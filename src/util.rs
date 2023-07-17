use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinSet;
use twilight_model::{id::{Id, marker::CommandMarker}, application::interaction::Interaction};

use crate::Context as BotContext;

pub async fn await_all<T: 'static>(set: &mut JoinSet<T>) {
    // Should probably return the error properly when I have time
    while let Some(_) = set.join_next().await {};
}

pub struct InteractionHelper {
    map: HashMap<Id<CommandMarker>, Handler>,
}
type Handler = Box<dyn (FnMut(&Arc<BotContext>, &Interaction) -> Result<(), ()>) + Send + Sync>;

impl InteractionHelper {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }
}
