use std::sync::Arc;

use serde::Deserialize;
use thiserror::Error;
use twilight_model::{channel::Message, application::interaction::Interaction};

use crate::Context as BotContext;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub channel_id: String,
    pub endpoint: String,
    pub token: String,
}

pub struct Context {}

#[derive(Error, Debug)]
pub enum MError {}

async fn expiry_task(ctx: Arc<BotContext>) {}

pub async fn init(ctx: &Arc<BotContext>) -> Result<(), MError> {
    tokio::spawn(expiry_task(ctx.clone()));
    Ok(())
}

pub async fn register_commands(ctx: &Arc<BotContext>) {
    /*let i9n_client = ctx.discord_client.interaction(ctx.application_id.load(atomic::Ordering::SeqCst).unwrap());
    let k = i9n_client.create_guild_command(ctx.config.home_guild).chat_input("bans", "Get bans list");*/
}

pub async fn handle_interaction(ctx: &Arc<BotContext>, interaction: &Interaction) {

}

pub fn handle_message(ctx: &Arc<BotContext>, message: &Message) {

}
