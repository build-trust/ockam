use ockam_command::docs::FencedCodeBlockHighlighter;
use ockam_command::docs::render;


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


#[cfg(test)]
mod tests_process_terminal_docs {
    use super::*;

    #[test]
    fn test_process_terminal_docs_with_code_blocks() {
        let input = "```sh
        # To enroll a known identity
        $ ockam project ticket --member id_identifier
        
        # To generate an enrollment ticket that can be used to enroll a device
        $ ockam project ticket --attribute component=control
        ```
        
        ";

        let result = render(input);

        assert!(result.contains("\x1b["), "The output should contain ANSI escape codes.");
        assert!(result.contains("\x1b[0m"), "The output should reset ANSI coloring at the end.");

        // Print the result to the terminal (stdout)
        println!("Highlighted text:\n\n{}", result);
    }
}