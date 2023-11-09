use crate::{Terminal, TerminalStream};
use console::Term;

pub enum DeleteMode {
    All,
    Selected(Vec<String>),
    Single(String),
    Default,
}

#[ockam_core::async_trait]
pub trait ShowItemsTui {
    const ITEM_NAME: &'static str;

    fn cmd_arg_item_name(&self) -> Option<&str>;
    fn terminal(&self) -> Terminal<TerminalStream<Term>>;
    async fn list_items_names(&self) -> miette::Result<Vec<String>>;
    async fn show_single(&self) -> miette::Result<()>;
    async fn show_multiple(&self, selected_items_names: Vec<String>) -> miette::Result<()>;

    async fn run(&self) -> miette::Result<()> {
        let terminal = self.terminal();
        if self.cmd_arg_item_name().is_some() || !terminal.can_ask_for_user_input() {
            self.show_single().await?;
            return Ok(());
        }

        let items_names = self.list_items_names().await?;
        match items_names.len() {
            0 => {
                terminal
                    .stdout()
                    .plain(format!("There are no {} to show", Self::ITEM_NAME))
                    .write_line()?;
            }
            1 => {
                self.show_single().await?;
            }
            _ => {
                let selected_item_names = terminal.select_multiple(
                    format!(
                        "Select one or more {} that you want to show",
                        Self::ITEM_NAME
                    ),
                    items_names,
                );
                match selected_item_names.len() {
                    0 => {
                        terminal
                            .stdout()
                            .plain(format!("No {} selected to show", Self::ITEM_NAME))
                            .write_line()?;
                    }
                    1 => {
                        self.show_single().await?;
                    }
                    _ => {
                        self.show_multiple(selected_item_names).await?;
                    }
                }
            }
        }
        Ok(())
    }
}
