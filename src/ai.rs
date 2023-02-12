use std::{
    cmp::min,
    collections::HashMap,
    fmt::{self, Display},
    iter::repeat,
    str::FromStr,
};
/// This struct constructs the prompt to send to OpenAi
/// The default implementation has good values for everything but
/// `language`, `git_diff` and `postmessage` although the `postmessage`
/// isn't really that important
#[derive(Debug)]
pub struct Prompt {
    /// The preamble (everything before the language) for the prompt
    pub preamble: Option<String>,
    /// The language **Please note this defaults to `python` if you dont change it
    pub language: Option<String>,
    /// Anything after the language and before the diff
    pub postamble: Option<String>,
    /// char that acts as a separator for the git diff, defaults to '='
    pub seperator: Option<char>,
    /// the actual git diff to analyze, this defaults to a silly python script
    pub git_diff: Option<String>,
    /// anything after the git diff
    pub postmessage: Option<String>,
}

/// Default implementation of the prompt
impl Default for Prompt {
    fn default() -> Self {
        Prompt {
            preamble: Some("Imagine you are an expert ".to_string()),
            language: Some("Python  ".to_string()),
            postamble: Some("developer and were given a git diff file to look at:".to_string()),
            git_diff: Some(DEFAULT_CODE.to_string()),
            seperator: Some('='),
            postmessage: Some(
                "Please write a paragraph summarizing the changes you made.".to_string(),
            ),
        }
    }
}

/// Display information for the prompt
impl Display for Prompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}\n{}\n{}\n{}\n{}",
            self.preamble.as_ref().unwrap_or(&"".to_string()),
            self.language.as_ref().unwrap_or(&"".to_string()),
            self.postamble.as_ref().unwrap_or(&"".to_string()),
            repeat(self.seperator.unwrap_or('='))
                .take(16)
                .collect::<String>(),
            self.git_diff.as_ref().unwrap_or(&"".to_string()),
            repeat(self.seperator.unwrap_or('='))
                .take(16)
                .collect::<String>(),
            self.postmessage.as_ref().unwrap_or(&"".to_string()),
        )
    }
}
// The request params to send to OpenAi for or completion
        prompt: Prompt,
        request_params.prompt = format!("{}", prompt);
        request_params.max_tokens = Some(min(
            <usize as TryInto<u16>>::try_into(request_params.prompt.chars().count()).unwrap() / 3,
            256,
        ));

const DEFAULT_CODE: &str = "
diff --git a/foo.py b/foo.py\n
new file mode 100644\n
index 0000000..e5a8e79\n
--- /dev/null\n
+++ b/foo.py\n
@@ -0,0 +1,5 @@\n
+def say_hi(name: str) -> str:\n
+    print(f'Hi {name}')\n
+\n
+if __name__ == 'main':\n
";