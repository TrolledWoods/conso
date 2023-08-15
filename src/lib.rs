//!
//! An experimental way to build command line interfaces, inspired by immediate mode guis.
//!
//!
//!

use std::io::Write;
use std::slice::Iter;
use std::ops::Range;
use std::str::FromStr;
use std::borrow::Cow;

/// Runs the parser on the command line arguments
pub fn args(handler: impl FnMut(&mut Ctx<'_, '_>)) {
    let args: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = args.iter().map(|v| &**v).collect();
    parse(&args, handler);
}

pub fn parse(segments: &[&str], mut handler: impl FnMut(&mut Ctx<'_, '_>)) {
    let mut finished = None;
    let mut input = Segments {
        iter: segments.iter(),
        depth: 0,
    };
    pick_sub_command(&mut input, &mut finished, &mut handler, true);
    if let Some(finished) = finished {
        print_finished_state(segments, finished);
    }
}

/// Queries for the user for input in a loop, until a command the user runs
/// asks the loop to quit.
pub fn user_loop<Result>(mut handler: impl FnMut(&mut Ctx<'_, '_, Result>)) -> Result {
    let mut input = String::new();
    loop {
        input.clear();
        print!("~> ");
        std::io::stdout().lock().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        let segments = input.split_whitespace().collect::<Vec<_>>();
        let mut input = Segments {
            iter: segments.iter(),
            depth: 0,
        };

        let mut finished = None;
        pick_sub_command(&mut input, &mut finished, &mut handler, true);
        if let Some(FinishedState::Okay(Some(result))) = finished {
            break result;
        }
        if let Some(finished) = finished {
            print_finished_state(&segments, finished);
        }
    }
}

fn print_finished_state<Result>(segments: &[&str], finished_state: FinishedState<Result>) {
    match finished_state {
        FinishedState::Okay(_) => {}
        FinishedState::Help { help } => {
            println!("# Help information");
            help.print_children(0);
        }
        FinishedState::Error { depth, message, help } => {
            println!("# Error");
            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", segment);
            }
            println!();

            for segment in segments.iter().take(depth as usize) {
                print!("{} ", segment);
            }
            println!("{} {}", "^".repeat(segments.get(depth as usize).map(|v| v.len()).unwrap_or(1)), message);

            if let Some(help) = help {
                print!("\nPotential inputs: ");
                help.print_children_tersely();
            }
        }
    }
}

fn pick_sub_command<'input, Result>(input: &mut Segments<'input>, finished: &mut Option<FinishedState<Result>>, mut handler: impl FnMut(&mut Ctx<'_, 'input, Result>), require_finish: bool) {
    if matches!(input.iter.as_slice(), ["help"]) {
        let mut help = HelpTree::default();
        let mut ctx = Ctx(CtxInner::BuildHelpInfo { help: &mut help });
        handler(&mut ctx);
        *finished = Some(FinishedState::Help { help });
        return;
    }

    let mut ctx = Ctx(CtxInner::PickCommand {
        input: input.clone(),
        finished,
    });
    handler(&mut ctx);

    if require_finish {
        if finished.is_none() {
            *finished = Some(FinishedState::Error {
                depth: input.depth,
                message: String::from("Input did not match any wanted command"),
                help: None,
            });
        }
    }

    // If we have an upstream error without any help, generate the full help
    // information
    if let Some(FinishedState::Error { help: help_opt @ None, .. }) = finished {
        let mut ctx = Ctx(CtxInner::BuildHelpInfo {
            help: help_opt.get_or_insert_with(Default::default),
        });
        handler(&mut ctx);
    }
}

#[derive(Clone)]
pub struct Segments<'a> {
    iter: Iter<'a, &'a str>,
    depth: u32,
}

impl<'a> Segments<'a> {
    fn next(&mut self) -> Option<&'a str> {
        match self.iter.next() {
            Some(v) => {
                self.depth += 1;
                Some(v)
            }
            None => {
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum FinishedState<Result> {
    Okay(Option<Result>),
    Help {
        help: HelpTree,
    },
    Error {
        depth: u32,
        message: String,
        help: Option<HelpTree>,
    },
}

/// The base struct to build "command trees".
pub struct Ctx<'r, 'input, Result = ()>(CtxInner<'r, 'input, Result>);

enum CtxInner<'r, 'input, Result> {
    PickCommand {
        input: Segments<'input>,
        finished: &'r mut Option<FinishedState<Result>>,
    },
    BuildHelpInfo {
        help: &'r mut HelpTree,
    },
}

impl<'input, Result> Ctx<'_, 'input, Result> {
    #[must_use = "Without using the return value, using this command will always yield an error"]
    pub fn command<C: Constraint>(&mut self, constraint: C) -> Command<'_, 'input, Result, C> {
        match &mut self.0 {
            CtxInner::PickCommand {
                input,
                finished,
            } => {
                let mut input = input.clone();
                match constraint.parse(&mut input) {
                    Some(data) => {
                        Command(CommandInner::PickCommand {
                            input,
                            data: Some(data),
                            finished,
                        })
                    }
                    None => {
                        Command(CommandInner::Skip)
                    }
                }
            }
            CtxInner::BuildHelpInfo {
                help,
            } => {
                help.branches.push(HelpTree::default());
                Command(CommandInner::BuildHelpInfo {
                    constraint,
                    help: help.branches.last_mut().expect("We just pushed a branch, it should exist"),
                })
            }
        }
    }
}

pub struct Command<'r, 'input, Result, C: Constraint>(CommandInner<'r, 'input, Result, C>);

enum CommandInner<'r, 'input, Result, C: Constraint> {
    PickCommand {
        input: Segments<'input>,
        data: Option<C::Output>,
        finished: &'r mut Option<FinishedState<Result>>,
    },
    Skip,
    BuildHelpInfo {
        constraint: C,
        help: &'r mut HelpTree,
    },
}

impl<'r, 'input, Result, C: Constraint> Command<'r, 'input, Result, C> {
    pub fn description(mut self, desc: &'static str) -> Self {
        match self.0 {
            CommandInner::BuildHelpInfo { ref mut help, .. } => {
                help.descriptions.push(Cow::Borrowed(desc));
            }
            _ => {}
        }

        self
    }

    pub fn sub_commands(mut self, mut handler: impl FnMut(&mut Ctx<'_, 'input, Result>)) -> Self {
        match &mut self.0 {
            CommandInner::PickCommand { input, finished, .. } => {
                pick_sub_command(input, *finished, handler, false);
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { help, .. } => {
                let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                    help,
                });
                handler(&mut ctx);
            }
        }

        self
    }

    pub fn arg<SubC: Constraint>(mut self, sub_c: SubC) -> Command<'r, 'input, Result, (C, SubC)> {
        match std::mem::replace(&mut self.0, CommandInner::Skip) {
            CommandInner::PickCommand { finished, data, mut input } => {
                if finished.is_none() {
                    let orig_depth = input.depth;
                    match sub_c.parse(&mut input) {
                        Some(new_data) => {
                            Command(CommandInner::PickCommand {
                                finished,
                                data: data.map(|data| (data, new_data)),
                                input,
                            })
                        }
                        None => {
                            *finished = Some(FinishedState::Error {
                                depth: orig_depth,
                                message: String::from("Invalid argument"),
                                help: None,
                            });

                            Command(CommandInner::PickCommand {
                                finished,
                                data: None,
                                input,
                            })
                        }
                    }
                } else {
                    Command(CommandInner::PickCommand {
                        finished,
                        data: None,
                        input,
                    })
                }
            }
            CommandInner::Skip => Command(CommandInner::Skip),
            CommandInner::BuildHelpInfo { help, constraint } => {
                Command(CommandInner::BuildHelpInfo {
                    help,
                    constraint: (constraint, sub_c),
                })
            }
        }
    }

    pub fn user_loop<SubResult>(mut self, mut handler: impl FnMut(&mut Ctx<'_, '_, SubResult>)) -> Self {
        match &mut self.0 {
            CommandInner::PickCommand { finished, data, input, .. } => {
                if finished.is_none() {
                    if input.iter.next().is_some() {
                        **finished = Some(FinishedState::Error {
                            depth: input.depth,
                            message: String::from("Excess arguments passed"),
                            help: None,
                        });
                        return self;
                    }

                    let mut input = String::new();
                    let result = loop {
                        input.clear();
                        print!("~> ");
                        std::io::stdout().lock().flush().unwrap();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let segments = input.split_whitespace().collect::<Vec<_>>();
                        let mut input = Segments {
                            iter: segments.iter(),
                            depth: 0,
                        };

                        let mut finished = None;
                        pick_sub_command(&mut input, &mut finished, &mut handler, true);
                        if let Some(FinishedState::Okay(Some(result))) = finished {
                            break result;
                        }
                        if let Some(finished) = finished {
                            print_finished_state(&segments, finished);
                        }
                    };

                    **finished = Some(FinishedState::Okay(None));
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { help, .. } => {
                help.descriptions.push(Cow::Borrowed("User loop"));
                help.is_standalone_command = true;
                let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                    help,
                });
                handler(&mut ctx);
            }
        }

        self
    }

    pub fn run_with(mut self, handler: impl FnOnce(&mut RunCtx<'_, C::Output, Result>)) -> Self {
        match &mut self.0 {
            CommandInner::PickCommand { finished, data, input, .. } => {
                if finished.is_none() {
                    if input.iter.next().is_some() {
                        **finished = Some(FinishedState::Error {
                            depth: input.depth,
                            message: String::from("Excess arguments passed"),
                            help: None,
                        });
                        return self;
                    }

                    let mut run_ctx = RunCtx {
                        data: data.as_ref().expect("If our data is none we should be in a finished state"),
                        result: None,
                    };
                    handler(&mut run_ctx);

                    **finished = Some(FinishedState::Okay(run_ctx.result));
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { help, .. } => {
                help.is_standalone_command = true;
            }
        }

        self
    }

    pub fn run(mut self, handler: impl FnOnce()) -> Self {
        self.run_with(|_| handler())
    }
}

impl<'input, Result, C: Constraint> Drop for Command<'_, 'input, Result, C> {
    fn drop(&mut self) {
        match &mut self.0 {
            CommandInner::PickCommand { input, finished, .. } => {
                if finished.is_none() {
                    **finished = Some(FinishedState::Error {
                        depth: input.depth,
                        message: String::from("Argument did not match any possible command"),
                        help: None,
                    });
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { constraint, help } => {
                constraint.extend_name(&mut |name| help.path_segment.push(name));
            }
        }
    }
}

pub struct RunCtx<'a, Data, Result> {
    data: &'a Data,
    result: Option<Result>,
}

impl<'a, Data, Result> RunCtx<'a, Data, Result> {
    pub fn data(&self) -> &'a Data {
        self.data
    }

    pub fn quit(&mut self, result: Result) {
        self.result = Some(result);
    }
}

#[derive(Default, Debug)]
pub struct HelpTree {
    path_segment: Vec<Cow<'static, str>>,
    descriptions: Vec<Cow<'static, str>>,
    branches: Vec<HelpTree>,
    is_standalone_command: bool,
}

fn print_indent(indent: u32) {
    for _ in 0..indent {
        print!(" |");
    }
}

fn print_indent_hook(indent: u32) {
    for _ in 1..indent {
        print!(" |");
    }

    if indent > 0 {
        print!(" +");
    }
}

impl HelpTree {
    fn print_name(&self, indent: u32) {
        print_indent_hook(indent);
        for (i, segment) in self.path_segment.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", segment);
        }
        println!();
    }

    fn print_children_tersely(&self) {
        for (i, branch) in self.branches.iter().enumerate() {
            if i > 0 {
                print!(" | ");
            }

            for (j, segment) in branch.path_segment.iter().enumerate() {
                if j > 0 {
                    print!(" ");
                }
                print!("{}", segment);
            }
        }

        println!();
    }

    fn print_children(&self, indent: u32) {
        if self.branches.is_empty() && !self.is_standalone_command {
            print_indent(indent);
            println!(" TODO: This command cannot be called...");
        }

        for description in &self.descriptions {
            for line in description.lines() {
                print_indent(indent);
                println!(" {}", line);
            }
        }

        for branch in &self.branches {
            branch.print_name(indent);
            branch.print_children(indent + 1);
        }
    }
}

pub trait Constraint {
    type Output;

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>));
    fn parse(self, input: &mut Segments<'_>) -> Option<Self::Output>;
}

pub struct InputString;

impl Constraint for InputString {
    type Output = String;

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>)) {
        callback(Cow::Borrowed("<string>"));
    }

    fn parse(self, input: &mut Segments<'_>) -> Option<Self::Output> {
        input.next().map(|v| v.to_string())
    }
}

impl Constraint for String {
    type Output = Self;

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>)) {
        callback(Cow::Owned(self.clone()));
    }

    fn parse(self, chunks: &mut Segments<'_>) -> Option<Self::Output> {
        (chunks.next() == Some(&&self)).then_some(self)
    }
}

impl Constraint for &'static str {
    type Output = Self;

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>)) {
        callback(Cow::Borrowed(self));
    }

    fn parse(self, chunks: &mut Segments<'_>) -> Option<Self::Output> {
        (chunks.next() == Some(&self)).then_some(self)
    }
}

impl<T> Constraint for Range<T>
where
    T: std::fmt::Display + FromStr + PartialOrd,
{
    type Output = T;

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>)) {
        callback(Cow::Owned(format!("number({}..{})", self.start, self.end)));
    }

    fn parse(self, chunks: &mut Segments<'_>) -> Option<Self::Output> {
        chunks.next()
            .and_then(|chunk| chunk.parse().ok())
            .filter(|v| self.contains(v))
    }
}

impl<A, B> Constraint for (A, B)
where
    A: Constraint,
    B: Constraint,
{
    type Output = (A::Output, B::Output);

    fn extend_name(&self, callback: &mut impl FnMut(Cow<'static, str>)) {
        let (a, b) = self;
        a.extend_name(callback);
        b.extend_name(callback);
    }

    fn parse(self, chunks: &mut Segments<'_>) -> Option<Self::Output> {
        let (a, b) = self;
        Some((a.parse(chunks)?, b.parse(chunks)?))
    }
}
