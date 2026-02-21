use anyhow::{Context, Result};
use std::fmt;

use crate::geometry::Geometry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionTarget {
    Output,
    Region,
    Window,
}

impl SelectionTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Region => "region",
            Self::Window => "window",
        }
    }
}

#[derive(Debug)]
pub(crate) enum SelectorError {
    Cancelled(SelectionTarget),
    Failed {
        target: SelectionTarget,
        message: String,
    },
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled(target) => write!(f, "slurp failed to select {}", target.as_str()),
            Self::Failed { target, message } => {
                write!(f, "slurp failed to select {}: {}", target.as_str(), message)
            }
        }
    }
}

impl std::error::Error for SelectorError {}

pub(crate) fn is_cancelled(err: &anyhow::Error, target: SelectionTarget) -> bool {
    err.downcast_ref::<SelectorError>()
        .is_some_and(|err| matches!(err, SelectorError::Cancelled(t) if *t == target))
}

fn cancelled_error(target: SelectionTarget) -> anyhow::Error {
    anyhow::Error::new(SelectorError::Cancelled(target))
}

fn selection_failed(target: SelectionTarget, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(SelectorError::Failed {
        target,
        message: message.into(),
    })
}

pub fn select_output(debug: bool) -> Result<Geometry> {
    let selection = slurp_rs::select_output(slurp_rs::SelectOptions::default())
        .map_err(|err| map_api_error(err, SelectionTarget::Output))?;
    let geometry = rect_to_geometry(&selection.rect)?;
    if debug {
        eprintln!("Output geometry: {}", geometry);
    }
    Ok(geometry)
}

pub fn select_region(debug: bool) -> Result<Geometry> {
    let options = slurp_rs::SelectOptions {
        display_dimensions: true,
        ..slurp_rs::SelectOptions::default()
    };
    let selection = slurp_rs::select_region(options)
        .map_err(|err| map_api_error(err, SelectionTarget::Region))?;
    let geometry = rect_to_geometry(&selection.rect)?;
    if debug {
        eprintln!("Region geometry: {}", geometry);
    }
    Ok(geometry)
}

pub fn select_from_boxes(boxes: &str, debug: bool) -> Result<Geometry> {
    let choices = parse_choice_boxes(boxes)?;
    let selection = slurp_rs::select_from_boxes(choices, slurp_rs::SelectOptions::default())
        .map_err(|err| map_api_error(err, SelectionTarget::Window))?;
    let geometry = rect_to_geometry(&selection.rect)?;
    if debug {
        eprintln!("Window geometry: {}", geometry);
    }
    Ok(geometry)
}

fn rect_to_geometry(rect: &slurp_rs::Rect) -> Result<Geometry> {
    Geometry::from_slurp_rect(rect)
}

pub(crate) fn parse_choice_boxes(input: &str) -> Result<Vec<slurp_rs::ChoiceBox>> {
    let mut out = Vec::new();
    for raw in input.lines() {
        let s = raw.trim_end_matches(['\n', '\r']);
        if s.trim().is_empty() {
            continue;
        }

        let first_ws = s
            .find(char::is_whitespace)
            .context("Invalid window box format: missing dimensions")?;
        let xy = &s[..first_ws];
        let mut rest = &s[first_ws..];
        rest = rest.trim_start();
        if rest.is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid window box format: empty dimensions"
            ));
        }

        let second_ws = rest.find(char::is_whitespace);
        let (wh, label) = match second_ws {
            Some(i) => {
                let label = rest[i..].trim_start();
                let label = if label.is_empty() {
                    None
                } else {
                    Some(label.to_string())
                };
                (&rest[..i], label)
            }
            None => (rest, None),
        };

        let (x, y) = parse_xy(xy).context("Invalid window box coordinates")?;
        let (width, height) = parse_wh(wh).context("Invalid window box dimensions")?;

        out.push(slurp_rs::ChoiceBox {
            rect: slurp_rs::Rect {
                x,
                y,
                width,
                height,
            },
            label,
            id: None,
        });
    }

    if out.is_empty() {
        return Err(anyhow::anyhow!("No valid windows found to capture"));
    }

    Ok(out)
}

fn parse_xy(value: &str) -> Option<(i32, i32)> {
    let (x, y) = value.split_once(',')?;
    let x = x.parse::<i32>().ok()?;
    let y = y.parse::<i32>().ok()?;
    Some((x, y))
}

fn parse_wh(value: &str) -> Option<(i32, i32)> {
    let (w, h) = value.split_once('x')?;
    let w = w.parse::<i32>().ok()?;
    let h = h.parse::<i32>().ok()?;
    Some((w, h))
}

pub(crate) fn map_api_error(err: slurp_rs::SlurpError, target: SelectionTarget) -> anyhow::Error {
    match err {
        slurp_rs::SlurpError::Cancelled => cancelled_error(target),
        _ => selection_failed(target, format!("slurp-rs: {err}")),
    }
}
