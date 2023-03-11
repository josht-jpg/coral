use std::{
    collections::HashMap,
    env,
    io::{self, BufRead, Read, Write},
    sync::Arc,
};

use anyhow::Result;
use browser::Browser;

use crate::window::create_window;

mod browser;
mod window;

fn main() {
    let browser = Browser::new();
}
