use std::collections::HashMap;

use crate::gamerunner::dispatcher::Message;

use super::statics::Statics;



pub struct State<'a> {
    pub handlebars: handlebars::Handlebars<'a>,
    pub statics: Statics, 
    pub channel: tokio::sync::mpsc::Sender<Message>, 
}