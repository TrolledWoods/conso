//!
//! A way to build command line interfaces, inspired by immediate mode guis.
//!
//! ```
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .run(|| {
//!             println!("Hello world!");
//!         });
//!
//!     ctx.command("order")
//!         .run(|| {
//!             println!("I would like a boiled crab, please");
//!         });
//! });
//! ```
//! In the above example our program can now run with three possible arguments;
//! * `greet`: This will print `Hello world!`
//! * `order`: This will print `I would like a boiled crab, please`
//! * `help`: This will print help information about the usage of the command.
//!
//! Notice how the help command is completely auto-generated!
//! We will also get nice error output if mistakes are found in the input.
//!
//! ## Usage
//! ### More help information
//! The names of commands may not be enough to describe what they do. Call [Command::description]
//! to add extra help information to a command.
//! ```
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .description("Give the world a wonderful greeting")
//!         .run(|| {
//!             println!("Hello world!");
//!         });
//!
//!     ctx.command("order")
//!         .description("Order something delicious")
//!         .run(|| {
//!             println!("I would like a boiled crab, please");
//!         });
//! });
//! ```
//!
//! ### Subcommands
//! Subcommands can be added by calling [Command::sub_commands]. This provides a new `ctx` that
//! can be used to add subcommands in the same way as normal commands.
//! ```
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .sub_commands(|ctx| {
//!             ctx.command("world")
//!                 .run(|| {
//!                     println!("Hello world!");
//!                 });
//!
//!             ctx.command("you")
//!                 .run(|| {
//!                     println!("Hello, you!");
//!                 });
//!         });
//! });
//! ```
//!
//! [Command::sub_commands] and [Command::run] can be combined. In this case,
//! the `run` will happen if no valid sub commands were found (and as long as there are no more
//! arguments given, if there were an error will be emitted instead).
//! ```
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .sub_commands(|ctx| {
//!             ctx.command("crudely")
//!                 .run(|| {
//!                     println!("Heyo world!");
//!                 });
//!         })
//!         .run(|| {
//!             println!("Hello world!");
//!         });
//! });
//! ```
//! 
//! Another way of acheiving the same thing is with [Ctx::otherwise].
//! ```
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .sub_commands(|ctx| {
//!             ctx.command("crudely")
//!                 .run(|| {
//!                     println!("Heyo world!");
//!                 });
//!
//!             ctx.otherwise()
//!                 .run(|| {
//!                     println!("Hello world!");
//!                 });
//!         });
//! });
//! ```
//!
//! ### Command groups
//! If there are a lot of commands and organization starts becoming necessary, it start becoming
//! necessary to bring out the big guns; good old functions!
//!
//! ```
//! fn greetings(ctx: &mut conso::Ctx) {
//!     ctx.command("crudely")
//!         // The help command is of course still automatically generated!
//!         .description("Crudely greet the world")
//!         .run(|| {
//!             println!("Heyo world!");
//!         });
//!
//!     ctx.otherwise()
//!         .run(|| {
//!             println!("Hello world!");
//!         });
//! }
//!
//! conso::args(|ctx| {
//!     ctx.command("greet")
//!         .sub_commands(greetings);
//!
//!     // This command also gets the same subcommands, but we also add an extra
//!     // one called `dont`.
//!     ctx.command("maybegreet")
//!         .sub_commands(|ctx| {
//!             ctx.command("dont")
//!                 .run(|| {});
//!             greetings(ctx);
//!         });
//! });
//! ```
//!
//! ### Interactivity
//! Sometimes just command line arguments aren't enough. We might want to allow the user to input
//! commands in a loop. As it happens [`user_loop`] exists just for this purpose!
//!
//! ```
//! conso::user_loop(|ctx, control_flow| {
//!     ctx.command("greet")
//!         .run(|| {
//!             println!("Hello world!");
//!         });
//!
//!     ctx.command("quit")
//!         .run(|| {
//!             control_flow.quit(());
//!         });
//! });
//! ```
//!
//! As opposed to [`args`], the closure here takes an extra argument called `control_flow`, that
//! lets you tell conso when the loop should be finished using `quit`. This also allows data to be
//! passed to the caller.
//!
//! ## Behind the scenes
//! The way this auto-generation works is a bit cheeky; and a hint can be found in the signature
//! of the [`args`] function:
//! ```
//! pub fn args(handler: impl FnMut(&mut Ctx<'_, '_>)) {
//!     todo!();
//! }
//! ```
//! Instead of taking an `FnOnce` closure like you might expect, it takes an `FnMut`. This lets
//! conso call it several times for different purposes. If the help command is called, or crono
//! wants to try and find suggested usages after an error has occured, conso will call this
//! function again, but in a special mode where nothing is really parsed and all `run` calls are
//! completely skipped.
//!
//! What does this mean in practice? Nothing much, mostly you get a really simple way to define
//! commands, while also getting nice help information for free! The main thing to keep in mind is
//! not to run complex logic without being inside of a `run` call, since that logic probably should
//! not run when `help` is called.
//!
//! The idea of the `control_flow` parameter inside of [`user_loop`] has a few big advantages;
//! one is that it allows [`Ctx`] to remain generic-less, which is a life-saver when grouping
//! commands together. It also allows nested user loops affect the control flows of parent user
//! loops easily. Originally the idea was to put a generic parameter on [`Ctx`] describing whether
//! it was a loop or not, and if it was a loop what return it had, but that was a pain so I'm much
//! happier with this approach. Originally [`Command`] and [`DataCommand`] were also going to be the same
//! type but with generics describing whether or not they had data attached, but that was scrapped
//! in favor of two types, with [`Command`] just being a thin wrapper over [`DataCommand`] instead.
//!

use std::io::Write;
use std::slice::Iter;
use std::ops::Range;
use std::str::FromStr;
use std::borrow::Cow;

/// Runs the parser on the command line arguments
pub fn args(handler: impl FnMut(&mut Ctx<'_, '_>)) {
    // HACK: It might be pretty bad to do skip(1) here actually.... it doesn't feel good..
    let args: Vec<String> = std::env::args().skip(1).collect();
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
pub fn user_loop<T>(mut handler: impl FnMut(&mut Ctx<'_, '_>, &mut ControlFlow<'_, T>)) -> T {
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

        let mut result = None;
        let mut control_flow = ControlFlow {
            result: Some(&mut result),
        };

        let mut finished = None;
        pick_sub_command(&mut input, &mut finished, |ctx| handler(ctx, &mut control_flow), true);
        if let Some(finished) = finished {
            print_finished_state(&segments, finished);
        }
        if let Some(result) = result {
            break result;
        }
    }
}

fn print_finished_state(segments: &[&str], finished_state: FinishedState) {
    match finished_state {
        FinishedState::Okay => {}
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

fn pick_sub_command<'input>(input: &mut Segments<'input>, finished: &mut Option<FinishedState>, mut handler: impl FnMut(&mut Ctx<'_, 'input>), require_finish: bool) {
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
pub enum FinishedState {
    Okay,
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
pub struct Ctx<'r, 'input>(CtxInner<'r, 'input>);

enum CtxInner<'r, 'input> {
    PickCommand {
        input: Segments<'input>,
        finished: &'r mut Option<FinishedState>,
    },
    BuildHelpInfo {
        help: &'r mut HelpTree,
    },
}

impl<'input> Ctx<'_, 'input> {
    pub fn otherwise(&mut self) -> Command<'_, 'input> {
        self.command(())
    }

    #[must_use = "Without using the return value, using this command will always yield an error"]
    pub fn data_command<C: Constraint>(&mut self, constraint: C) -> DataCommand<'_, 'input, C::Output> {
        match &mut self.0 {
            CtxInner::PickCommand {
                input,
                finished,
            } => {
                let mut input = input.clone();
                match constraint.parse(&mut input) {
                    Some(data) => {
                        DataCommand(CommandInner::PickCommand {
                            input,
                            data: Some(data),
                            finished,
                        })
                    }
                    None => {
                        DataCommand(CommandInner::Skip)
                    }
                }
            }
            CtxInner::BuildHelpInfo {
                help,
            } => {
                help.branches.push(HelpTree::default());
                let sub_help = help.branches.last_mut().expect("We just pushed a branch, it should exist");
                constraint.extend_name(&mut |part| sub_help.path_segment.push(part));
                DataCommand(CommandInner::BuildHelpInfo {
                    help: sub_help,
                })
            }
        }
    }

    #[must_use = "Without using the return value, using this command will always yield an error"]
    pub fn command<C: Constraint>(&mut self, constraint: C) -> Command<'_, 'input> {
        Command(self.data_command(constraint).map(|_| ()))
    }
}

pub struct Command<'r, 'input>(DataCommand<'r, 'input, ()>);

pub struct DataCommand<'r, 'input, T>(CommandInner<'r, 'input, T>);

enum CommandInner<'r, 'input, T> {
    PickCommand {
        input: Segments<'input>,
        data: Option<T>,
        finished: &'r mut Option<FinishedState>,
    },
    Skip,
    BuildHelpInfo {
        help: &'r mut HelpTree,
    },
}

impl<'r, 'input> Command<'r, 'input> {
    pub fn description(mut self, desc: &'static str) -> Self {
        match self.0.0 {
            CommandInner::BuildHelpInfo { ref mut help, .. } => {
                help.descriptions.push(Cow::Borrowed(desc));
            }
            _ => {}
        }

        self
    }

    pub fn sub_commands(mut self, mut handler: impl FnMut(&mut Ctx<'_, 'input>)) -> Self {
        match &mut self.0.0 {
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

    pub fn user_loop(mut self, mut handler: impl FnMut(&mut Ctx<'_, '_>, &mut ControlFlow<'_, ()>)) {
        match &mut self.0.0 {
            CommandInner::PickCommand { finished, input, .. } => {
                if finished.is_none() {
                    if input.iter.next().is_some() {
                        **finished = Some(FinishedState::Error {
                            depth: input.depth,
                            message: String::from("Excess arguments passed"),
                            help: None,
                        });
                    }

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

                        let mut result = None;
                        let mut control_flow = ControlFlow {
                            result: Some(&mut result),
                        };

                        let mut finished = None;
                        pick_sub_command(&mut input, &mut finished, |ctx| handler(ctx, &mut control_flow), true);
                        if let Some(finished) = finished {
                            print_finished_state(&segments, finished);
                        }

                        if result.is_some() {
                            break;
                        }
                    };

                    **finished = Some(FinishedState::Okay);
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { help, .. } => {
                help.descriptions.push(Cow::Borrowed("User loop"));
                help.is_standalone_command = true;
                let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                    help,
                });
                handler(&mut ctx, &mut ControlFlow { result: None });
            }
        }
    }

    pub fn arg<SubC: Constraint>(self, sub_c: SubC) -> DataCommand<'r, 'input, SubC::Output> {
        self.0.arg(sub_c).map(|(_, v)| v)
    }

    pub fn run(self, handler: impl FnOnce()) {
        self.0.run(|()| handler());
    }
}

impl<'r, 'input, T> DataCommand<'r, 'input, T> {
    pub fn description(mut self, desc: &'static str) -> Self {
        match self.0 {
            CommandInner::BuildHelpInfo { ref mut help, .. } => {
                help.descriptions.push(Cow::Borrowed(desc));
            }
            _ => {}
        }

        self
    }

    fn map<OutT>(mut self, mapper: impl FnOnce(T) -> OutT) -> DataCommand<'r, 'input, OutT> {
        match std::mem::replace(&mut self.0, CommandInner::Skip) {
            CommandInner::PickCommand { input, data, finished } => {
                DataCommand(CommandInner::PickCommand {
                    input,
                    data: data.map(mapper),
                    finished,
                })
            }
            CommandInner::Skip => DataCommand(CommandInner::Skip),
            CommandInner::BuildHelpInfo { help } => {
                DataCommand(CommandInner::BuildHelpInfo {
                    help,
                })
            }
        }
    }

    pub fn arg<SubC: Constraint>(mut self, sub_c: SubC) -> DataCommand<'r, 'input, (T, SubC::Output)> {
        match std::mem::replace(&mut self.0, CommandInner::Skip) {
            CommandInner::PickCommand { finished, data, mut input } => {
                if finished.is_none() {
                    let orig_depth = input.depth;
                    match sub_c.parse(&mut input) {
                        Some(new_data) => {
                            DataCommand(CommandInner::PickCommand {
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

                            DataCommand(CommandInner::PickCommand {
                                finished,
                                data: None,
                                input,
                            })
                        }
                    }
                } else {
                    DataCommand(CommandInner::PickCommand {
                        finished,
                        data: None,
                        input,
                    })
                }
            }
            CommandInner::Skip => DataCommand(CommandInner::Skip),
            CommandInner::BuildHelpInfo { help } => {
                sub_c.extend_name(&mut |part| help.path_segment.push(part));
                DataCommand(CommandInner::BuildHelpInfo {
                    help,
                })
            }
        }
    }

    pub fn run(mut self, handler: impl FnOnce(&T)) {
        match &mut self.0 {
            CommandInner::PickCommand { finished, data, input, .. } => {
                if finished.is_none() {
                    if input.iter.next().is_some() {
                        **finished = Some(FinishedState::Error {
                            depth: input.depth,
                            message: String::from("Excess arguments passed"),
                            help: None,
                        });
                        return;
                    }

                    handler(data.as_ref().expect("If our data is none we should be in a finished state"));

                    **finished = Some(FinishedState::Okay);
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildHelpInfo { help } => {
                help.is_standalone_command = true;
            }
        }
    }
}

impl<'input, T> Drop for DataCommand<'_, 'input, T> {
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
            CommandInner::BuildHelpInfo { .. } => {}
        }
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

pub struct ControlFlow<'a, T> {
    result: Option<&'a mut Option<T>>,
}

impl<T> ControlFlow<'_, T> {
    pub fn quit(&mut self, value: T) {
        if let Some(result) = &mut self.result {
            **result = Some(value);
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

impl Constraint for () {
    type Output = ();

    fn extend_name(&self, _callback: &mut impl FnMut(Cow<'static, str>)) {}

    fn parse(self, _input: &mut Segments<'_>) -> Option<Self::Output> {
        Some(())
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
