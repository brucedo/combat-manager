use std::collections::HashMap;



pub struct State<'a> {
    pub handlebars: handlebars::Handlebars<'a>,
    pub statics: HashMap<String, String>
}