#![doc = include_str!("../README.md")]

use std::io::Write;
use std::slice::Iter;
use std::ops::Range;
use std::str::FromStr;

/// Runs the parser on the command line arguments
pub fn args(handler: impl FnMut(&mut Ctx<'_, '_>)) {
    // HACK: It might be pretty bad to do skip(1) here actually.... it doesn't feel good..
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(|v| &**v).collect();
    parse(&args, handler);
}

pub fn parse(segments: &[&str], mut handler: impl FnMut(&mut Ctx<'_, '_>)) {
    match &*segments {
        ["help"] => {
            let mut help = HelpFmt::default();
            let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                help: &mut help,
            });
            handler(&mut ctx);
            help.line_break();
        }
        ["help", segments @ ..] => {
            let mut help = HelpFmt::default();
            let mut finished = None;
            Command(DataCommand(CommandInner::BuildSubHelpInfo {
                input: Segments {
                    original: segments,
                    iter: segments.iter(),
                    depth: 0,
                },
                help: &mut help,
                finished: &mut finished,
            })).sub_commands(handler);
            help.line_break();
            if let Some(finished) = finished {
                print_finished_state(&segments, finished);
            }
        }
        segments => {
            let mut input = Segments {
                original: &segments,
                iter: segments.iter(),
                depth: 0,
            };
            let mut finished = None;
            pick_sub_command(&mut input, &mut finished, handler, true);
            if let Some(finished) = finished {
                print_finished_state(&segments, finished);
            }
        }
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
        let mut result = None;
        parse(&segments, |ctx| handler(ctx, &mut ControlFlow { result: Some(&mut result) }));
        if let Some(result) = result {
            break result;
        }
    }
}

fn print_finished_state(segments: &[&str], finished_state: FinishedState) {
    match finished_state {
        FinishedState::Okay => {}
        FinishedState::Help => {},
        FinishedState::Error { depth, message, help } => {
            println!("# Error");
            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", segment);
            }
            println!();

            let length = segments.iter().take(depth as usize).map(|segment| segment.len() + 1).sum::<usize>();
            println!("{}{} {}", " ".repeat(length), "^".repeat(segments.get(depth as usize).map(|v| v.len()).unwrap_or(1)), message);

            if let Some(help) = help {
                print!("\nUsage: \n");
                print!("{}", help);
            }
        }
    }
}

fn pick_sub_command<'input>(input: &mut Segments<'input>, finished: &mut Option<FinishedState>, mut handler: impl FnMut(&mut Ctx<'_, 'input>), require_finish: bool) {
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
    if let Some(FinishedState::Error { depth, help: help_opt @ None, .. }) = finished {
        let mut help = HelpFmt {
            output: Some(String::new()),
            ..Default::default()
        };

        if *depth == input.depth {
            let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                help: &mut help,
            });
            handler(&mut ctx);
        } else {
            for part in &input.original[.. *depth as usize] {
                help.push_word(part);
            }
            help.indent();

            let mut sub_finished = None;
            let sub_segments = &input.original[input.depth as usize .. *depth as usize];
            let sub_input = Segments {
                original: sub_segments,
                iter: sub_segments.iter(),
                depth: 0,
            };
            let mut ctx = Ctx(CtxInner::BuildSubHelpInfo {
                input: sub_input,
                finished: &mut sub_finished,
                help: &mut help,
            });
            handler(&mut ctx);
        }

        help.line_break();

        *help_opt = help.output.take();
    }
}

#[derive(Clone)]
pub struct Segments<'a> {
    original: &'a [&'a str],
    iter: Iter<'a, &'a str>,
    depth: u32,
}

impl<'a> Segments<'a> {
    pub fn finished(&self) -> bool {
        self.iter.as_slice().is_empty()
    }

    pub fn next(&mut self) -> Option<&'a str> {
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
enum FinishedState {
    Okay,
    Help,
    Error {
        depth: u32,
        message: String,
        help: Option<String>,
    },
}

/// The base struct to build "command trees".
pub struct Ctx<'r, 'input>(CtxInner<'r, 'input>);

enum CtxInner<'r, 'input> {
    PickCommand {
        input: Segments<'input>,
        finished: &'r mut Option<FinishedState>,
    },
    BuildSubHelpInfo {
        input: Segments<'input>,
        help: &'r mut HelpFmt,
        finished: &'r mut Option<FinishedState>,
    },
    BuildHelpInfo {
        help: &'r mut HelpFmt,
    },
}

impl<'input> Ctx<'_, 'input> {
    pub fn otherwise(&mut self) -> Command<'_, 'input> {
        self.command(())
    }

    #[must_use = "Without using the return value, using this command will always yield an error"]
    pub fn command<C: ConstrainedArg<'input>>(&mut self, constraint: C) -> Command<'_, 'input> {
        Command(self.data_command(constraint).map(|_| ()))
    }

    #[must_use = "Without using the return value, using this command will always yield an error"]
    pub fn data_command<C: ConstrainedArg<'input>>(&mut self, constraint: C) -> DataCommand<'_, 'input, C::Output> {
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
            CtxInner::BuildSubHelpInfo {
                input,
                finished,
                help,
            } => {
                let mut input = input.clone();
                if finished.is_none() && constraint.parse(&mut input).is_some() {
                    if input.finished() {
                        **finished = Some(FinishedState::Help);

                        DataCommand(CommandInner::BuildHelpInfo {
                            help,
                        })
                    } else {
                        DataCommand(CommandInner::BuildSubHelpInfo {
                            input,
                            finished,
                            help,
                        })
                    }
                } else {
                    DataCommand(CommandInner::Skip)
                }
            }
            CtxInner::BuildHelpInfo {
                help,
            } => {
                constraint.help(help);
                help.indent();
                DataCommand(CommandInner::BuildHelpInfo {
                    help,
                })
            }
        }
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
    BuildSubHelpInfo {
        input: Segments<'input>,
        help: &'r mut HelpFmt,
        finished: &'r mut Option<FinishedState>,
    },
    BuildHelpInfo {
        help: &'r mut HelpFmt,
    },
}

impl<'r, 'input> Command<'r, 'input> {
    pub fn description(self, desc: &'static str) -> Self {
        Command(self.0.description(desc))
    }

    pub fn sub_commands(mut self, mut handler: impl FnMut(&mut Ctx<'_, 'input>)) -> Self {
        match &mut self.0.0 {
            CommandInner::PickCommand { input, finished, .. } => {
                pick_sub_command(input, *finished, handler, false);
            }
            CommandInner::Skip => {}
            CommandInner::BuildSubHelpInfo { input, finished, help } => {
                if finished.is_some() {
                    return self;
                }

                if input.finished() {
                    let mut ctx = Ctx(CtxInner::BuildHelpInfo {
                        help: &mut **help,
                    });
                    handler(&mut ctx);
                    **finished = Some(FinishedState::Help);
                } else {
                    let mut ctx = Ctx(CtxInner::BuildSubHelpInfo {
                        input: input.clone(),
                        finished: &mut **finished,
                        help: &mut **help,
                    });
                    handler(&mut ctx);
                }
            }
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

                    user_loop(handler);
                    **finished = Some(FinishedState::Okay);
                }
            }
            CommandInner::Skip => {}
            CommandInner::BuildSubHelpInfo { input, help, finished } => {
                if finished.is_none() {
                    let mut ctx = Ctx(CtxInner::BuildSubHelpInfo {
                        input: input.clone(),
                        finished,
                        help: &mut **help,
                    });
                    handler(&mut ctx, &mut ControlFlow { result: None });
                }
            }
            CommandInner::BuildHelpInfo { help, .. } => {
                help.push_paragraph("User loop");
            }
        }
    }

    pub fn arg<T: Arg<'input>>(self) -> DataCommand<'r, 'input, T> {
        self.constrained_arg(unconstrained::<T>())
    }

    pub fn constrained_arg<SubC: ConstrainedArg<'input>>(self, sub_c: SubC) -> DataCommand<'r, 'input, SubC::Output> {
        self.0.constrained_arg(sub_c).map(|(_, v)| v)
    }

    pub fn run(self, handler: impl FnOnce()) {
        self.0.run(|()| handler());
    }
}

impl<'r, 'input, T> DataCommand<'r, 'input, T> {
    pub fn description(mut self, desc: &'static str) -> Self {
        match self.0 {
            CommandInner::BuildHelpInfo { ref mut help, .. } => {
                help.small_indent();
                help.push_paragraph(desc);
                help.small_deindent();
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
            CommandInner::BuildSubHelpInfo { input, help, finished } => {
                DataCommand(CommandInner::BuildSubHelpInfo {
                    input,
                    help,
                    finished,
                })
            }
            CommandInner::BuildHelpInfo { help } => {
                DataCommand(CommandInner::BuildHelpInfo {
                    help,
                })
            }
        }
    }

    pub fn arg<V: Arg<'input>>(self) -> DataCommand<'r, 'input, (T, V)> {
        self.constrained_arg(unconstrained::<V>())
    }

    pub fn constrained_arg<SubC: ConstrainedArg<'input>>(mut self, sub_c: SubC) -> DataCommand<'r, 'input, (T, SubC::Output)> {
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
            CommandInner::BuildSubHelpInfo { mut input, help, finished } => {
                if finished.is_none() {
                    let orig_depth = input.depth;
                    match sub_c.parse(&mut input) {
                        Some(_) => {
                            DataCommand(CommandInner::BuildSubHelpInfo {
                                help,
                                finished,
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
                    DataCommand(CommandInner::BuildSubHelpInfo {
                        finished,
                        help,
                        input,
                    })
                }
            }
            CommandInner::BuildHelpInfo { help } => {
                help.indent();
                help.push_word("Argument:");
                sub_c.help(help);
                help.deindent();
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
            CommandInner::BuildSubHelpInfo { .. } => {}
            CommandInner::BuildHelpInfo { .. } => {}
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
            CommandInner::BuildSubHelpInfo { input, finished, .. } => {
                if finished.is_none() {
                    **finished = Some(FinishedState::Error {
                        depth: input.depth,
                        message: String::from("Argument did not match any possible command"),
                        help: None,
                    });
                }
            }
            CommandInner::BuildHelpInfo { help } => {
                help.deindent();
            }
        }
    }
}

pub struct HelpFmt {
    indent: u32,
    small_indent: u32,
    indent_str: &'static str,
    current_line_length: usize,
    max_length: usize,
    empty_line: bool,
    output: Option<String>,
}

impl Default for HelpFmt {
    fn default() -> Self {
        Self {
            indent: 0,
            small_indent: 0,
            indent_str: " | ",
            current_line_length: 0,
            max_length: 100,
            empty_line: true,
            output: None,
        }
    }
}

impl HelpFmt {
    fn push_completely_raw(&mut self, stuff: &str) {
        match self.output {
            Some(ref mut string) => string.push_str(stuff),
            None => print!("{}", stuff),
        }
    }

    fn print_indent(&mut self) {
        self.empty_line = false;
        for _ in 0..self.indent {
            self.push_completely_raw(self.indent_str);
            self.current_line_length += self.indent_str.len();
        }

        for _ in 0..self.small_indent {
            self.push_completely_raw(" ");
            self.current_line_length += 1;
        }
    }

    pub fn indent(&mut self) {
        self.indent += 1;
        self.small_indent = 0;
        self.line_break();
    }

    pub fn deindent(&mut self) {
        if self.indent != 0 {
            self.indent -= 1;
            self.small_indent = 0;
        }
        self.line_break();
    }

    pub fn small_indent(&mut self) {
        self.small_indent += 1;
        self.line_break();
    }

    pub fn small_deindent(&mut self) {
        if self.small_indent != 0 {
            self.small_indent -= 1;
        }
        self.line_break();
    }

    pub fn push_raw_str(&mut self, string: &str) {
        if self.empty_line {
            self.print_indent();
        }

        self.push_completely_raw(string);
        self.current_line_length += self.indent_str.len();
    }

    pub fn push_word(&mut self, word: &str) {
        if !self.empty_line {
            if self.current_line_length + word.len() > self.max_length {
                self.line_break();
            } else {
                self.push_raw_str(" ");
            }
        }

        self.push_raw_str(word);
    }

    pub fn push_paragraph(&mut self, string: &str) {
        for (i, line) in string.lines().enumerate() {
            if i > 0 {
                self.line_break();
            }

            for word in line.split_whitespace() {
                self.push_word(word);
            }
        }
    }

    pub fn line_break(&mut self) {
        if !self.empty_line {
            self.push_completely_raw("\n");
            self.empty_line = true;
            self.current_line_length = 0;
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

pub trait Arg<'a> {
    fn help(fmt: &mut HelpFmt);
    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized;
}

impl<'a, T: Arg<'a>> Arg<'a> for Option<T> {
    fn help(fmt: &mut HelpFmt) {
        fmt.push_word("(");
        T::help(fmt);
        fmt.push_word(")?");
    }

    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized {
        let old_segments = input.clone();
        match T::parse(input) {
            Some(v) => {
                Some(Some(v))
            }
            None => {
                *input = old_segments;
                Some(None)
            }
        }
    }
}

impl<'a, T: Arg<'a>> Arg<'a> for Vec<T> {
    fn help(fmt: &mut HelpFmt) {
        fmt.push_word("(");
        T::help(fmt);
        fmt.push_word(")*");
    }

    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized {
        let vector = std::iter::from_fn(|| T::parse(input)).collect::<Vec<_>>();
        Some(vector)
    }
}

impl<'a, const N: usize, T: Arg<'a>> Arg<'a> for [T; N] {
    fn help(fmt: &mut HelpFmt) {
        for _ in 0..N {
            T::help(fmt);
        }
    }

    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized {
        let vector = (0..N).map(|_| T::parse(input)).collect::<Option<Vec<_>>>()?;
        vector.try_into().ok()
    }
}

impl<'a> Arg<'a> for &'a str {
    fn help(fmt: &mut HelpFmt) {
        fmt.push_word("<string>");
    }

    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized {
        input.next()
    }
}

impl<'a> Arg<'a> for String {
    fn help(fmt: &mut HelpFmt) {
        fmt.push_word("<string>");
    }

    fn parse(input: &mut Segments<'a>) -> Option<Self> where Self: Sized {
        input.next().map(String::from)
    }
}

pub trait ConstrainedArg<'a> {
    type Output;

    fn help(&self, fmt: &mut HelpFmt);
    fn parse(&self, input: &mut Segments<'a>) -> Option<Self::Output>;
}

pub fn either<A, B>(a: A, b: B) -> Either<A, B> {
    Either(a, b)
}

pub struct Either<A, B>(A, B);

impl<'a, A, B> ConstrainedArg<'a> for Either<A, B>
where
    A: ConstrainedArg<'a>,
    B: ConstrainedArg<'a, Output = A::Output>,
{
    type Output = A::Output;

    fn help(&self, fmt: &mut HelpFmt) {
        fmt.push_word("[");
        let Either(a, b) = self;
        a.help(fmt);
        fmt.push_word("|");
        b.help(fmt);
        fmt.push_word("]");
    }

    fn parse(&self, input: &mut Segments<'a>) -> Option<Self::Output> {
        let Either(a, b) = self;

        {
            let mut temp = input.clone();
            if let Some(result) = a.parse(&mut temp) {
                *input = temp;
                return Some(result);
            }
        }

        {
            let mut temp = input.clone();
            if let Some(result) = b.parse(&mut temp) {
                *input = temp;
                return Some(result);
            }
        }

        None
    }
}

impl<'a> ConstrainedArg<'a> for String {
    type Output = ();

    fn help(&self, fmt: &mut HelpFmt) {
        fmt.push_word(&self);
    }

    fn parse(&self, chunks: &mut Segments<'a>) -> Option<Self::Output> {
        (chunks.next() == Some(self)).then_some(())
    }
}

impl<'a> ConstrainedArg<'a> for &str {
    type Output = ();

    fn help(&self, fmt: &mut HelpFmt) {
        fmt.push_word(&self);
    }

    fn parse(&self, chunks: &mut Segments<'a>) -> Option<Self::Output> {
        (chunks.next() == Some(&self)).then_some(())
    }
}

impl<'a, T> ConstrainedArg<'a> for Range<T>
where
    T: std::fmt::Display + FromStr + PartialOrd,
{
    type Output = T;

    fn help(&self, fmt: &mut HelpFmt) {
        fmt.push_word(&format!("<number {}..{}>", self.start, self.end));
    }

    fn parse(&self, chunks: &mut Segments<'a>) -> Option<Self::Output> {
        chunks.next()
            .and_then(|chunk| chunk.parse().ok())
            .filter(|v| self.contains(v))
    }
}

macro_rules! impl_tuples {
    ($($n:ident: $t:ident),*) => {
        #[allow(warnings)]
        impl<'a, $($t: ConstrainedArg<'a>),*> ConstrainedArg<'a> for ($($t,)*) {
            type Output = ($($t::Output,)*);

            fn help(&self, fmt: &mut HelpFmt) {
                let ($($n,)*) = self;
                $(
                    $n.help(fmt);
                )*
            }

            fn parse(&self, chunks: &mut Segments<'a>) -> Option<Self::Output> {
                let ($($n,)*) = self;
                $(
                    let $n = $n.parse(chunks)?;
                )*
                Some(($($n,)*))
            }
        }

        #[allow(warnings)]
        impl<'a, $($t: Arg<'a>),*> Arg<'a> for ($($t,)*) {
            fn help(fmt: &mut HelpFmt) {
                $(
                    $t::help(fmt);
                )*
            }

            fn parse(chunks: &mut Segments<'a>) -> Option<Self> {
                $(
                    let $n = $t::parse(chunks)?;
                )*
                Some(($($n,)*))
            }
        }
    }
}

impl_tuples!(a: A, b: B, c: C, d: D, e: E, f: F);
impl_tuples!(a: A, b: B, c: C, d: D, e: E);
impl_tuples!(a: A, b: B, c: C, d: D);
impl_tuples!(a: A, b: B, c: C);
impl_tuples!(a: A, b: B);
impl_tuples!(a: A);
impl_tuples!();

pub struct Unconstrained<T>(std::marker::PhantomData<T>);

pub fn unconstrained<T>() -> Unconstrained<T> {
    Unconstrained(std::marker::PhantomData)
}

impl<'a, T> ConstrainedArg<'a> for Unconstrained<T>
where
    T: Arg<'a>,
{
    type Output = T;

    fn help(&self, fmt: &mut HelpFmt) {
        <T as Arg>::help(fmt);
    }

    fn parse(&self, input: &mut Segments<'a>) -> Option<Self::Output> {
        <T as Arg>::parse(input)
    }
}
