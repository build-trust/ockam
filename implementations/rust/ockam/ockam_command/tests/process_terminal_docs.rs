use ockam_command::docs::FencedCodeBlockHighlighter;

#[cfg(test)]
mod test_fenced_code_block_highlighter {
    use super::*;

    #[test]
    fn test_syntax_highlighting() {
        let mut highlighter = FencedCodeBlockHighlighter::new();
        let mut output = Vec::new();

        // Simulate the start of a code block
        assert!(highlighter.process_line("```sh\n", &mut output));

        // Simulate processing a line of code within the code block
        let code_line = "echo \"Hello, world!\"\n";
        let highlighted = highlighter.process_line(code_line, &mut output);
        
        // We expect this line to be processed (highlighted)
        assert!(highlighted);

        // The output should contain the syntax highlighted version of the code line
        // This is a simplistic check for ANSI escape codes - your actual check might be more complex
        assert!(output.last().unwrap().contains("\x1b["));

        // Simulate the end of a code block
        assert!(highlighter.process_line("```\n", &mut output));
        
        // Check that the highlighting is reset at the end
        assert!(output.last().unwrap().contains("\x1b[0m"));

        // Print the highlighted output
        for line in &output {
            println!("{}", line);
        }
    }
}


