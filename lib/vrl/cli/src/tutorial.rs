use super::{open_url, Error, Repl};
use rustyline::{error::ReadlineError, Editor};
use serde::Deserialize;
use vrl::{diagnostic::Formatter, state, Runtime, Target, Value};

#[derive(Deserialize)]
struct Tutorial {
    section: usize,
    id: usize,
    title: String,
    help_text: String,
    // The URL endpoint (https://vrl.dev/:endpoint) for finding out more
    docs: String,
    correct_answer: Value,
    initial_event: Value,
}

impl Tutorial {
    fn number(&self) -> String {
        format!("{}.{}", self.section, self.id)
    }
}

#[derive(Deserialize)]
struct Tutorials {
    tutorials: Vec<Tutorial>,
}

pub fn tutorial() -> Result<(), Error> {
    let mut index = 0;
    let mut compiler_state = state::Compiler::default();
    let mut rt = Runtime::new(state::Runtime::default());
    let mut rl = Editor::<Repl>::new();
    rl.set_helper(Some(Repl::new("> ")));

    let mut tutorials = load_tutorials_from_toml().tutorials;

    // Tutorial intro
    clear_screen();
    println!("Welcome to the Vector Remap Language interactive tutorial!\n");
    print_tutorial_help_text(0, &tutorials);

    'outer: loop {
        let readline = rl.readline("> ");
        match readline.as_deref() {
            Ok(line) if line == "exit" || line == "quit" => break 'outer,
            Ok(line) => {
                rl.add_history_entry(line);

                match line {
                    "" => continue,
                    "help" => help(),
                    "next" => {
                        clear_screen();

                        // End if no more tutorials are left, or else increment the index
                        if (index + 1) == tutorials.len() {
                            println!("\n\nCongratulations! You've successfully completed the VRL tutorial.\n");
                            break;
                        } else {
                            index = index.saturating_add(1);
                        }

                        print_tutorial_help_text(index, &tutorials);
                    }
                    "prev" => {
                        clear_screen();

                        if index == 0 {
                            println!("\n\nYou're back at the beginning!\n\n");
                        }

                        index = index.saturating_sub(1);
                        print_tutorial_help_text(index, &tutorials);
                    }
                    "docs" => {
                        let tut = &tutorials[index];
                        let endpoint = &tut.docs;
                        let docs_url = format!("https://vrl.dev/{}", endpoint);

                        open_url(&docs_url);

                        clear_screen();
                    }
                    command => {
                        let tut = &mut tutorials[index];
                        let event = &mut tut.initial_event;
                        let correct_answer = &tut.correct_answer;

                        // Purely for debugging
                        if command == "cheat" {
                            clear_screen();
                            println!("{}", correct_answer);
                        }

                        match resolve_to_value(event, &mut rt, command, &mut compiler_state) {
                            Ok(result) => {
                                if event == correct_answer {
                                    clear_screen();

                                    println!(
                                        "CORRECT! You've wisely ended up with this event:\n\n{}\n",
                                        event
                                    );

                                    // Exit if no more tutorials are left, otherwise move on to the next one
                                    if (index + 1) == tutorials.len() {
                                        println!("Congratulations! You've successfully completed the VRL tutorial.\n");
                                        break 'outer;
                                    } else {
                                        println!(
                                            "You've now completed tutorial {} out of {}.\nType `next` and hit Enter to move on to tutorial number {} or `exit` to leave the VRL tutorial.\n",
                                            index + 1,
                                            tutorials.len(),
                                            index + 2,
                                        );

                                        // Wait for "next" to continue
                                        {
                                            let mut rl = Editor::<Repl>::new();

                                            'next: loop {
                                                match rl.readline("> ").as_deref() {
                                                    Ok(line)
                                                        if line == "exit" || line == "quit" =>
                                                    {
                                                        break 'outer
                                                    }
                                                    Ok(line) if line == "next" => {
                                                        clear_screen();
                                                        break 'next;
                                                    }
                                                    _ => {
                                                        println!("\nDidn't recognize that input. Type `next` and hit Enter to move on or `exit` to leave the VRL tutorial.\n");
                                                        continue;
                                                    }
                                                }
                                            }
                                        }

                                        index = index.saturating_add(1);
                                        print_tutorial_help_text(index, &tutorials);
                                    }
                                } else {
                                    println!("{}", result);
                                }
                            }
                            Err(err) => {
                                println!("{}", err);
                            }
                        }
                    }
                };
            }
            Err(ReadlineError::Interrupted) => break 'outer,
            Err(ReadlineError::Eof) => break 'outer,
            Err(err) => {
                println!("unable to read line: {}", err);
                break 'outer;
            }
        }
    }

    Ok(())
}

fn help() {
    println!("{}", HELP_TEXT);
}

fn print_tutorial_help_text(index: usize, tutorials: &[Tutorial]) {
    let tut = &tutorials[index];

    println!(
        "Tutorial {}: {}\n\n{}\nInitial event object:\n{}\n",
        tut.number(), tut.title, tut.help_text, tut.initial_event
    );
}

fn load_tutorials_from_toml() -> Tutorials {
    let toml_file = std::include_str!("../tutorials.toml");

    toml::from_str(toml_file).unwrap()
}

#[cfg(unix)]
fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}

#[cfg(windows)]
fn clear_screen() {
    print!("\x1b[2J");
}

// This function reworks the resolve function in repl.rs to return a Result rather than a String. If the Result is
// Ok, the value is used to check whether the current event is equal to the "correct" answer.
pub fn resolve_to_value(
    object: &mut dyn Target,
    runtime: &mut Runtime,
    program: &str,
    state: &mut state::Compiler,
) -> Result<Value, String> {
    let program = match vrl::compile_with_state(program, &stdlib::all(), state) {
        Ok(program) => program,
        Err(diagnostics) => {
            let msg = Formatter::new(program, diagnostics).colored().to_string();
            return Err(msg);
        }
    };

    match runtime.resolve(object, &program) {
        Ok(v) => Ok(v),
        Err(err) => Err(err.to_string()),
    }
}

// Help text
const HELP_TEXT: &str = r#"
Tutorial commands:
  next     Load the next tutorial
  prev     Load the previous tutorial
  exit     Exit the VRL interactive tutorial
  cheat    Choose the coward's way out
"#;