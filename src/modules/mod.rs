use std::sync::Arc;

use futures::Future;
use serde::Deserialize;
use tokio::{sync::RwLock, task::{JoinHandle, JoinSet}};
use twilight_model::application::interaction::Interaction;

use crate::{Context as BotContext, util, BotState};

mod database;
mod ddnet_bans;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub database: Option<database::Config>,
    pub ddnet_bans: Option<ddnet_bans::Config>,
}

pub struct Context {
    pub task_joinset: JoinSet<()>,
    pub database: Option<database::Context>,
    pub ddnet_bans: Option<ddnet_bans::Context>,
}

pub async fn init_modules(context: &Arc<BotContext>) -> anyhow::Result<()> {
    let mut lock = context.modules.write().await;
    let ctx = &mut *lock;
    *ctx = Some(Context {
        task_joinset: JoinSet::new(),
        database: None,
        ddnet_bans: None,
    });
    drop(lock);

    database::init(context).await?;
    ddnet_bans::init(context).await?;

    Ok(())
}

pub async fn register_commands(context: Arc<BotContext>) {
    assert!(context.application_id.load(atomic::Ordering::SeqCst).is_some());
    ddnet_bans::register_commands(&context).await;
}

pub async fn handle_interaction(context: Arc<BotContext>, interaction: Interaction) {
    ddnet_bans::handle_interaction(&context, &interaction).await;
}

pub async fn get_task_joinset(context: &Arc<BotContext>) -> JoinSet<()> {
    let mut lock = context.modules.write().await;
    return std::mem::take(&mut lock.as_mut().unwrap().task_joinset);
}

pub async fn create_task<T>(context: &Arc<BotContext>, task: T) -> Result<(), ()>
where
    T: Future<Output = ()> + Send + 'static
{
    if matches!(context.state.load(atomic::Ordering::SeqCst), BotState::Terminating{..}) {
        return Err(());
    }

    // Care for deadlock
    let mut lock = context.modules.write().await;
    lock.as_mut().unwrap().task_joinset.spawn(task);

    Ok(())
}
