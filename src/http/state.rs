use std::collections::HashMap;

use super::statics::Statics;



pub struct State<'a> {
    pub handlebars: handlebars::Handlebars<'a>,
    pub statics: Statics
}