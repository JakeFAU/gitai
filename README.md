# GitAI or How I Stopped Writing Commit Messages

This is a little project combining two hot things right now: [ChatGPT](https://chat.openai.com/chat) and [Rust](https://www.rust-lang.org/). I've been meaning to learn Rust for some time now, and with the *wonder* that is ChatGPT I found the perfect project to try.

If you just want to look at the code its [here](https://github.com/JakeFAU/gitai) but please consider two things

1. This is my first Rust program and even I can see the difference between some of the early code I wrote and the later stuff.
2. This is a work in progress, not everything is implemented yet.

## WARNING

1. OpenAI is not free, you are going to have to sign up for an API key.  They give you a reasonable amount of credits, and they make it really easy to set a max spend.  Please note, your humble servant makes nothing, OpenAI doesn't have an affiliate program (Note to self, ask OpenAI about an affiliate program)

2. If you use it at your job, you are almost certainly violating your employment agreement.  This program sends Git Diff files to OpenAI and therefore your code (imagine adding a new file) becomes part of its brain.

You know its funny we (not me Jose) all *"leak"* information to [Google](https://www.google.com/) and [Stack Overflow](https://stackoverflow.com/) all the time.  Not secrets (although that happens) but information about your code (tell me you've never copy/pasted an error message).  This somehow feels worse, and not only because its information from your git repository (who owns [GitHub](https://github.com/)?).  I think its because the quality of the answers makes it scary, and that I'm sure is why Google is worried.

### On To The Code

Ok, like all good code users, I'm going to look at the help message:

```bash
Usage: gitai [OPTIONS] [COMMAND]

Commands:
  commit  Generate Commit Message
  pr      Generare Pull Request
  models  Get AI Models - Good for testing connectivity
  help    Print this message or the help of the given subcommand(s)

Options:
      --git_api_token <GITHUB_TOKEN>
          set GitHub API token
      --git_api_url <GITHUB_URL>
          set GitHub API url
      --ai_api_token <AI_TOKEN>
          set OpenAI token
      --ai_api_url <AI_URL>
          set OpenAI url
  -c, --config <FILE>
          Sets a custom config file
  -l, --local-repo <REPO>
          Sets a custom local repo, you should probably not use this
  -v, --verbose
          Turn Verbose Mode on
  -s, --stochastic
          Turn Stochastic Mode on
  -a, --auto-add
          Turns Auto Add mode on which adds . to git before making the commit DANGEROUS
  -i, --auto-ai
          Turns Auto AI mode on automatically accepts the AI message without review DANGEROUS
  -n, --num-tries <TRIES>
          Number of times to try the AI: Note OpenAI Chatbot is not Idenpotent
  -g, --gpg-sign-commit
          Sign Commits, if set some variables must be added to settings.json
  -p, --programming-language <LANGUAGE>
          Programming Language, very useful for small commits/pr
      --signature-id <SIGNATURE_ID>
          Signing Key ID: Note, ignored if sign_commit=false
  -h, --help
          Print help
  -V, --version
          Print version
```

There sure are a lot of options, someone has been busy.  Not all of them are implemented yet, nor has the `PR` command.  However the `commit` command works, and I was so excited I couldn't wait to show it off.

It is important to note that most of this can be set in a `settings.json` file in `$HOME/.gitai` and this program will put a blank one there if it doesn't exist.

- git_api_token: Pretty obvious, not needed for commits
- git_api_url: Same
- ai_api_token: Again obvious, but this needs to be set for commits
- ai_api_url: Same
- config: If you dont want to use `$HOME/.gitai/settings.json as your config file, you can point it elsewhere here
- local-repo: If you dont want to run this at `.` you can point this to another Git Repo. I used this for testing, you probably shouldn't.
- verbose: Come on
- stochastic: Comming soon!
- auto-add: This is the equivalent of running `git add .` Not that anyone does that :)
- auto-ai: Will automatically accept the AI message without review.  In other words if you run `gitai -a -i commit` you are letting the machine make all your decisions
- num-tries: Comming soon!
- gpg-sign-commit: Coming soon!
- programming-language: Lets ChatGPT know what programming language is the predominant one in this commit.  This is really helpful for small commits, where it would be tough to guess the language.
- signature-id: The id of the signature key you want to use.

Now if you notice the gpg stuff can also be set in your git settings `commit.gpgsign` and `user.signingkey` are the keys, and gitai will read from there as well.

So how does it work?  Well I gave it this git diff file (this is the equivalent of running the command `git diff --cached`) if you want to see your own diff file).

```git
diff --git a/src/ai.rs b/src/ai.rs
    index ea36b54..07a8d0d 100644
    --- a/src/ai.rs
    +++ b/src/ai.rs
    @@ -149,14 +149,14 @@ impl Default for OpenAiRequestParams {
     149                 .expect("Why cant I set the default?"),
     150             suffix: None,
     151             max_tokens: Some(256),
    -152             temperature: Some(0.0),
    +0             temperature: Some(0.05),
     153             top_p: Some(1.0),
     154             n: Some(1),
     155             logprobs: None,
     156             echo: Some(false),
     157             stop: None,
    -158             presence_penalty: Some(0.0),
    -159             frequency_penalty: Some(0.0),
    +0             presence_penalty: Some(0.2),
    +0             frequency_penalty: Some(0.2),
     160             best_of: Some(1),
     161         }
     162     }
    @@ -249,9 +249,10 @@ impl OpenAiClient {
     249         let mut request_params = open_ai_request_params.unwrap_or_default();
     250         request_params.prompt = format!("{}", prompt);
     251         request_params.max_tokens = Some(min(
    -252             <usize as TryInto<u16>>::try_into(request_params.prompt.chars().count()).unwrap() / 3,
    +0             <usize as TryInto<u16>>::try_into(request_params.prompt.chars().count()).unwrap() / 10,
     253             256,
     254         ));
    +0         debug!("Max Tokens Set To {}", &request_params.max_tokens.unwrap());
     255         let res = self.client.post(url).json(&request_params).send()?;
     256         let data = res.json::<OpenAiCompletionResponse>()?;
     257         return Ok(data);
```

and it came back with the very reasonable:

```markdown
## Solution
The developer changed the default values of temperature, presence_penalty, and frequency_penalty from 0.0 to 0.05, 0.2, and 0.2 respectively.
```

#### Thoughts on Rust

Rust has a very steep learning curve, especially for someone who has never coded in C or C++.  Ownership and lifetimes are particularly troubling at first.  It does become more natural as you go further, and then you can see the real power of Rust.

I would compare it to my favorite language [Go](https://go.dev/), but it would be incredibly biased, as I have been paid to write Go code, and this is my first Rust project.

I will say this the toolset of the modern languages (Go and Rust) is amazing.  There is no reason you should be coding in any other language unless its legacy code.  Your productivity and the quality of you work will thank you.

- There are two obvious exceptions, Python for Data Science and JavaScript (please use TypeScript) for front-ends. Next time you are asked to write yet another flask based, containerized API, just say, "Maybe we should use [Gin](https://github.com/gin-gonic/gin)" instead".  You'll thank me.

I think my favorite thing about Rust is the `Option` and `Result` types (and the fact that you can write test cases in your documentation and Rust will run them.  Whoever thought of that is a genius)

```Rust
    let user_email = match settings["git_information"]["options"]["user_email"].as_str() {
        Some(email) => email.to_string(),
        None => git_config.get_string("user.email")?.to_string(),
    };
```

Look how elegant that is, its possible because of those two return types. For those who dont know rust this command looks in the settings for that multilevel key.  If it finds it, it sets `user_email` to it, if not, it looks in the Git Config.

#### Thoughts on AI

As you can imagine I now have some intimate knowledge of cutting edge AI completion engines, and it has really changed my opinion.

First, Google is in trouble.  After a couple of days coding this, I rarely looked at Google (or StackOverflow), I would just ask ChatGPT my question and it would give me an answer.

Google has won for years because Google > Bing, certainly by enough that its not worth changing any default behaivor to Bing.  The problem for Google is that Bing + ChatGPT has the potential to be so much better (The implementation will be tough, but if they get it right there is no reason to use Google).

Second, I am less impressed (worried) about AI then I was before (I don't work for Google).  At first I was seduced by the *magic* of ChatGPT but as I played with it more I think I have come to understand it.

I was worried because when you first play with ChatGPT it seems **intellegent** but as you use it more, you can see what it is really doing (or what I think anyway)

Imagine you had a table with every possible hash-code and what you should return if someone prompts you with the hashcode.  No I'm not in the mood to figure out if there is enough room in the universe to keep track of all these, but you can see the impracticality of it (Mathamaticians have all the fun, nobody tells them that number can't fit in the universe).

ChatGPT is then better thought of as minimizing the number of hashcodes by smartly (using math and programming that I'm sure is beyond me) tokenizing your sentances and minimizing the number of hashcodes to search.

Perhaps an example might make it clearer.  Suppose you had the following two sentences

1. That meal was superb.
2. That meal was excellent.

Now OpenAI's tokenizer which was trained on what must be billions of text documents, has *learned* in this context "superb" and "excellent" mean the same thing, so you have eliminated one hashcode you need in your table.  Do this enough times, with enough data, and smart enough people (and again, the implementation is, I have no doubt, brilliant).

However when you think like this, you can see that ChatGPT isn't doing anything more then running trillions of correlations to shrink the search space for your tokenized prompt.

I have also come to think about what must make us (or helps make us) intelligent; feedback loops.  What you notice when you play with these completion engines enough, is they fail badly.  When ChatGPT gives you the right answer it seems like magic, but the engine will return gibberish on occasion.

This is easier to see with [DALL-E](https://labs.openai.com/) which works in a similar way with images (its their tokenizer thats amazing).

I asked it a simple question *"Create a photo of a person enjoying a big dish of jellybeans"*

The first one that came back is nothing short of amazing
![Amazing](img/amazing.png). You can really see why people are so excited about this tech.  However, even though this looks like it could have easily come from a human, lets look at the next two images (DALL-E creates four):
![Huh1](img/huh1.png)
![Huh2](img/huh2.png)
I dont know what these mini people are doing with the jellybeans but I don't think they are enjoying them.

However that doesn't prepare you for the horror of:
![Horror](img/Scary.png)
I dont know if thats a person, if those are jelly beans, and how on earth is he enjoying them.

Now if you were to ask a graphics designer the same question, they might come back with something like the first one. There is no way they would come back with the next two, not to mention the horror that is the last one.

Somewhere in our brains we must create feedback loops, that prevent us from going this far off the rails. Feedback loops that we have not let learned to put in our AI models yet.

I have no doubt that ChatGPT is only going to get better and tools like it are going to become a more important part of our lives.  However, I don't think they are **Intelligent** yet, just a much better search engine with an amazing tokenizer.

One day we might learn to create those feedback loops, and then we might really be in trouble (I for one welcome our silicon overlords). Until then, I think humanity is pretty safe.
