// This program creates and then immediately stops a node.

use ockam::{node, Context, Result};
use r3bl_ansi_color::{AnsiStyledText, Color, Style};

#[rustfmt::skip]
const HELP_TEXT: &str =r#"
┌───────────────────────┐
│  Node 1               │
├───────────────────────┤
│  ┌─────────────────┐  │
│  │ Worker Address: │  │
│  │ 'app'           │  │
│  └─────────────────┘  │
└───────────────────────┘
"#;

/// Create and then immediately stop a node.
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    AnsiStyledText {
        text: HELP_TEXT,
        style: &[Style::Foreground(Color::Rgb(100, 200, 0))],
    }
    .println();

    print_title(vec!["Run a node & stop it right away"]);

    // Create a node.
    let mut node = node(ctx);

    // Stop the node as soon as it starts.
    node.stop().await
}

fn print_title(title: Vec<&str>) {
    let msg = format!("🚀 {}", title.join("\n  → "));
    AnsiStyledText {
        text: msg.as_str(),
        style: &[
            Style::Bold,
            Style::Foreground(Color::Rgb(70, 70, 70)),
            Style::Background(Color::Rgb(100, 200, 0)),
        ],
    }
    .println();
}
