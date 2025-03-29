use crate::command::Commands;

#[derive(Clone, Debug)]
pub struct OutputBranding {
    pub brand_name: String,
    pub bin_name: String,
    pub commands: Commands,
}

impl OutputBranding {
    pub fn new(brand_name: String, bin_name: String, commands: Commands) -> Self {
        Self {
            brand_name,
            bin_name,
            commands,
        }
    }

    pub fn replace(&self, text: &str) -> String {
        // brand name
        let mut text = if self.brand_name != "Ockam" {
            text.replace("Ockam", &self.brand_name)
        } else {
            text.to_string()
        };
        // command names
        for command in &self.commands.commands {
            text = text.replace(
                &format!("ockam {}", command.name),
                &format!("ockam {}", command.custom_name),
            );
        }
        // bin name
        text = if self.bin_name != "ockam" {
            text.replace("ockam", &self.bin_name)
        } else {
            text
        };
        text
    }
}

impl Default for OutputBranding {
    fn default() -> Self {
        Self {
            brand_name: "Ockam".to_string(),
            bin_name: "ockam".to_string(),
            commands: Commands::new(""),
        }
    }
}
