mod config;
mod lexer;
mod modules;
mod util;

use atomic::Atomic;
use futures::{StreamExt, future::join};
use reqwest::Client as HttpClient;
use tokio::{sync::{broadcast, RwLock}, task::{JoinHandle, JoinSet}};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, util::SubscriberInitExt};
use twilight_gateway::{
    stream::{self, ShardEventStream},
    Config as TWConfig, Intents, Event,
};
use twilight_http::Client as DiscordClient;
use twilight_model::id::{Id, marker::{UserMarker, ApplicationMarker}};

use std::{path::Path, sync::{Arc, atomic::{AtomicI32, Ordering}}};
use tracing::{info, error, warn, debug};

use config::Config;
use modules::Context as ModuleContext;

#[derive(Clone, Copy)]
pub enum BotState {
    Running,
    Terminating{ soft: bool },
}

pub struct Context {
    state: Atomic<BotState>,
    config: Config,
    c_token: CancellationToken,
    discord_client: DiscordClient,
    http: HttpClient,

    bot_id: Atomic<Option<Id<UserMarker>>>,
    application_id: Atomic<Option<Id<ApplicationMarker>>>,

    modules: RwLock<Option<ModuleContext>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder().with_env_filter(EnvFilter::from_default_env()).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let c = config::parse_config(Path::new("config.toml"))?;

    info!(?c);

    let discord_http = DiscordClient::new(c.token.clone());
    let discord_config = TWConfig::new(
        c.token.clone(),
        Intents::GUILDS | Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS | Intents::MESSAGE_CONTENT,
    );
    let mut discord_shards =
        stream::create_recommended(&discord_http, discord_config, |_, b| b.build())
            .await?
            .collect::<Vec<_>>();

    let mut discord_stream = ShardEventStream::new(discord_shards.iter_mut());

    let context = Arc::new(Context {
        state: Atomic::new(BotState::Running),
        config: c,
        c_token: CancellationToken::new(),
        discord_client: discord_http,
        http: HttpClient::new(),
        bot_id: Atomic::new(None),
        application_id: Atomic::new(None),
        modules: RwLock::new(None),
    });

    let mut event_handlers: JoinSet<()> = JoinSet::new();

    modules::init_modules(&context).await?;

    let mut c_count = 0;
    loop {
        // These SeqCst operations could perhaps be Acq-Rel instead
        if matches!(context.state.load(Ordering::SeqCst), BotState::Terminating{..}) {
            break;
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                c_count += 1;
                info!("^C received, terminating gracefully.");
                terminate(&context, true);
            },
            Some((_, event)) = discord_stream.next() => {
                match event {
                    Ok(e) => handle_discord_event(&context, &e),
                    Err(err) => {
                        warn!("ReceiveMessageError: {:?}", err);
                        if err.is_fatal() {
                            error!("Previous error was fatal, terminating.");
                            terminate(&context, true);
                        }
                    }
                }
            }
        }
    }

    let prev_c_count = c_count;
    let mut tasks = modules::get_task_joinset(&context).await;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                c_count += 1;
                match c_count - prev_c_count {
                    1 => {
                        info!("^C received, terminating gracefully.");
                    },
                    2 => {
                        info!("^C received, next ^C will cause a hard termination with possible data loss.");
                    },
                    3 => {
                        terminate(&context, false);
                        info!("^C received, terminating.");
                        break;
                    },
                    _ => unreachable!()
                }
            },
            _ = join(util::await_all(&mut event_handlers), util::await_all(&mut tasks)) => {
                break;
            }
        }
    }

    Ok(())
}

fn terminate(context: &Arc<Context>, soft: bool) {
    context.state.store(BotState::Terminating{soft}, Ordering::SeqCst);
    context.c_token.cancel();
}

// Spawn a new task if doing anything that takes significant time
fn handle_discord_event(context: &Arc<Context>, event: &Event) {
    match event {
        Event::Ready(r) => {
            info!("Ready, in {} guilds", r.guilds.len());
            if !r.guilds.iter().map(|u| { u.id }).any(|i| { i == context.config.home_guild }) {
                warn!("Bot is not in home guild: {}", context.config.home_guild);
            }

            debug!("User ID: {}, Application ID: {}", r.user.id, r.application.id);
            context.bot_id.store(Some(r.user.id), Ordering::SeqCst);
            context.application_id.store(Some(r.application.id), Ordering::SeqCst);

            tokio::spawn(modules::register_commands(context.clone()));
        },
        Event::MessageCreate(m) => {
            info!("M|{}: {}", m.author.name, m.content);
        },
        Event::InteractionCreate(i) => {
            tokio::spawn(modules::handle_interaction(context.clone(), i.0.clone()));
        },
        e => { debug!("Unhandled event {:?}", e); }
    }
}
