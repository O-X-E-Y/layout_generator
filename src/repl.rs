use std::io::Write;
use clap::{arg, command, Command};
use crate::generate::LayoutGeneration;
use crate::generate::BasicLayout;
use crate::analyze::{Config, Weights};

pub struct Repl {
    language: String,
    gen: LayoutGeneration,
    weights: Weights,
    pins: Vec<usize>
}

impl Repl {
    pub fn run() -> Result<(), String> {
        let config = Config::new();

        let mut env = Self {
            language: config.defaults.language.clone(),
            gen: LayoutGeneration::new(
                config.defaults.language.as_str(),
                config.weights.clone()
            ),
            weights: config.weights,
            pins: config.pins
        };

        loop {
            let line = readline()?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match env.respond(line) {
                Ok(quit) => {
                    if quit {
                        break;
                    }
                }
                Err(err) => {
                    write!(std::io::stdout(), "{}", err).map_err(|e| e.to_string())?;
                    std::io::stdout().flush().map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }

    fn get_nth(&self, nr: usize) -> Option<BasicLayout> {
        if let Some(temp_list) = &self.gen.temp_generated {
            if nr < temp_list.len() {
                Some(BasicLayout::try_from(temp_list[nr].as_str()).unwrap())
            } else {
                println!("That's not a valid index!");
                None
            }
        } else {
            println!("You haven't generated any layouts yet!");
            None
        }
    }

    fn save(&mut self, save_m: &clap::ArgMatches) {
        let n_str = save_m.value_of("NR").unwrap();
        if let Ok(nr) = usize::from_str_radix(n_str, 10) {
            if let Some(layout) = self.get_nth(nr) {
                if let Some(name) = save_m.value_of("NAME") {
                    self.gen.analysis.save(layout, Some(name.to_string())).unwrap();
                } else {
                    self.gen.analysis.save(layout, None).unwrap();
                }
            }
        }
    }

    fn respond(&mut self, line: &str) -> Result<bool, String> {
        let args = shlex::split(line).ok_or("error: Invalid quoting")?;
        let matches = self.cli()
            .try_get_matches_from(&args)
            .map_err(|e| e.to_string())?;
        match matches.subcommand() {
            Some(("generate", new_m)) => {
                let count_str = new_m.value_of("COUNT").unwrap();
                println!("generating {} layouts...", count_str);
                let count = usize::from_str_radix(count_str, 10).map_err(|e| e.to_string())?;
                self.gen.generate_n(count);
            }
            Some(("improve", comp_m)) => {
                let name = comp_m.value_of("LAYOUT_NAME").unwrap();
                let amount_str = comp_m.value_of("AMOUNT").unwrap();
                if let Ok(amount) = usize::from_str_radix(amount_str, 10) {
                    if let Some(l) = self.gen.analysis.layout_by_name(name) {
                        self.gen.generate_n_pins(amount, l.clone(), &self.pins);
                    }
                }
            }
            Some(("rank", _)) => {
                self.gen.analysis.rank();
            }
            Some(("layout", new_m)) => {
                let name_or_nr = new_m.value_of("LAYOUT_NAME_OR_NR").unwrap();
                if let Ok(nr) = usize::from_str_radix(name_or_nr, 10) {
                    if let Some(layout) = self.get_nth(nr) {
                        self.gen.analysis.analyze(&layout);
                    }
                } else {
                    self.gen.analysis.analyze_name(name_or_nr);
                }
            }
            Some(("compare", new_m)) => {
                let layout1 = new_m.value_of("LAYOUT_1").unwrap();
                let layout2 = new_m.value_of("LAYOUT_2").unwrap();
                self.gen.analysis.compare_name(layout1, layout2);
            }
            Some(("language", lang_m)) => {
                match lang_m.value_of("LANGUAGE") {
                    Some(language) => {
                        self.language = language.to_string();
                        self.gen = LayoutGeneration::new(language, self.weights.clone());
                        println!("Set language to {}", language);
                    },
                    None => println!("Current language: {}", self.language)
                }
            }
            Some(("languages", _)) => {
                for entry in std::fs::read_dir("static/language_data").unwrap() {
                    if let Ok(p) = entry {
                        let name = p
                            .file_name()
                            .to_string_lossy()
                            .replace("_", " ")
                            .replace(".json", "");
                        if name != "test" {
                            println!("{}", name);
                        }
                    }
                }
            }
            Some(("reload", _)) => {
                let new_config = Config::new();
                self.gen = LayoutGeneration::new(self.language.as_str(), new_config.weights);
                self.pins = new_config.pins;
            }
            Some(("save", save_m)) => {
                self.save(save_m);
            }
            Some(("quit", _)) => {
                println!("Exiting anlyzer...");
                return Ok(true);
            }
            Some((name, _new_m)) => unimplemented!("{}", name),
            None => unreachable!("subcommand required"),
        }

        Ok(false)
    }

    fn cli(&self) -> Command<'static> {
        // strip out usage
        const PARSER_TEMPLATE: &str = "\
            {all-args}
        ";
        // strip out name/version
        const APPLET_TEMPLATE: &str = "\
            {about-with-newline}\n\
            {usage-heading}\n    {usage}\n\
            \n\
            {all-args}{after-help}\
        ";

        command!("repl")
            .multicall(true)
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommand_value_name("APPLET")
            .subcommand_help_heading("APPLETS")
            .help_template(PARSER_TEMPLATE)
            .subcommand(
                command!("rank")
                    .alias("r")
                    .alias("sort")
                    .about("Rank all layouts in set language by score")
                    .help_template(APPLET_TEMPLATE),
            )
            .subcommand(
                command!("layout")
                    .alias("l")
                    .alias("analyze")
                    .alias("a")
                    .arg(
                        arg!(<LAYOUT_NAME_OR_NR>)
                    )
                    .about("Show details of layout")
                    .help_template(APPLET_TEMPLATE)
            )
            .subcommand(
                command!("compare")
                    .alias("c")
                    .arg(
                        arg!(<LAYOUT_1>)
                    )
                    .arg(
                        arg!(<LAYOUT_2>)
                    )
                    .about("Compare 2 layouts")
                    .help_template(APPLET_TEMPLATE)
            )
            .subcommand(
                command!("language")
                    .alias("lang")
                    .alias("lanugage")
                    .alias("langauge")
                    .arg(   
                        arg!([LANGUAGE])
                    )
                    .help_template(APPLET_TEMPLATE)
                    .about("Set a language to be used for analysis. Loads corpus when not present")
            )
            .subcommand(
                command!("languages")
                .help_template(APPLET_TEMPLATE)
                .about("Show available languages")
            )
            .subcommand(
                command!("reload")
                .alias("r")
                .help_template(APPLET_TEMPLATE)
                .about("Reloads all data with the current language. Loses temporary layouts.")
            )
            .subcommand(
                command!("generate")
                    .alias("gen")
                    .arg(
                        arg!(<COUNT>)
                    )
                    .help_template(APPLET_TEMPLATE)
                    .about("Generate a number of layouts and take the best 10")
            )
            .subcommand(
                command!("improve")
                    .alias("i")
                    .alias("optimize")
                    .arg(
                        arg!(<LAYOUT_NAME>)
                    )
                    .arg(
                        arg!(<AMOUNT>)
                    )
                    .help_template(APPLET_TEMPLATE)
                    .about("Save the top <NR> result that was generated. Starts from 1, takes negative values")
            )
            .subcommand(
                command!("save")
                .arg(
                    arg!(<NR>)
                )
                .arg(
                    arg!([NAME])
                )
                .help_template(APPLET_TEMPLATE)
                .about("Save the top <NR> result that was generated. Starts from 1, takes negative values")
            )
            .subcommand(
                command!("quit")
                    .alias("exit")
                    .about("Quit the repl")
                    .help_template(APPLET_TEMPLATE),
            )
    }
}

fn readline() -> Result<String, String> {
    write!(std::io::stdout(), "> ").map_err(|e| e.to_string())?;
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}