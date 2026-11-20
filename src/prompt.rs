use crate::error::usage;
use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use std::io::IsTerminal;

pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

/// Yes/no question. `yes` (flag `--yes`) answers it without asking; without a TTY it is an error.
pub fn confirm(msg: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !interactive() {
        return Err(usage(format!(
            "confirmation required ({msg}) – without a terminal please pass --yes"
        )));
    }
    Ok(Confirm::new()
        .with_prompt(msg)
        .default(false)
        .interact_opt()?
        .unwrap_or(false))
}

pub fn select(msg: &str, items: &[String], default: usize) -> Result<usize> {
    select_with(msg, items, default, true)
}

/// Like `select`, but leaves no “<question>: <answer>” line behind after the choice –
/// for callers that print the result themselves anyway.
pub fn select_silent(msg: &str, items: &[String], default: usize) -> Result<usize> {
    select_with(msg, items, default, false)
}

fn select_with(msg: &str, items: &[String], default: usize, report: bool) -> Result<usize> {
    if !interactive() {
        return Err(usage(format!("a choice is required ({msg}), but there is no terminal")));
    }
    Select::new()
        .with_prompt(msg)
        .items(items)
        .default(default)
        .report(report)
        .interact_opt()?
        .ok_or_else(|| usage("aborted"))
}

/// Input with validation; asks again on an invalid value.
pub fn input_validated(
    msg: &str,
    default: Option<&str>,
    check: impl Fn(&String) -> std::result::Result<(), String>,
) -> Result<String> {
    if !interactive() {
        return Err(usage(format!("input required ({msg}), but there is no terminal")));
    }
    let mut input = Input::<String>::new().with_prompt(msg).validate_with(check);
    if let Some(d) = default {
        input = input.default(d.to_string());
    }
    Ok(input.interact_text()?.trim().to_string())
}

pub fn input(msg: &str) -> Result<String> {
    if !interactive() {
        return Err(usage(format!("input required ({msg}), but there is no terminal")));
    }
    let s: String = Input::new().with_prompt(msg).interact_text()?;
    Ok(s.trim().to_string())
}
