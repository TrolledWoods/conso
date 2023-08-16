A way to build command line interfaces, inspired by immediate mode guis.

```rust
conso::args(|ctx| {
    ctx.command("greet")
        .run(|| {
            println!("Hello world!");
        });

    ctx.command("order")
        .run(|| {
            println!("I would like a boiled crab, please");
        });
});
```
In the above example our program can now run with three possible arguments;
* `greet`: This will print `Hello world!`
* `order`: This will print `I would like a boiled crab, please`
* `help`: This will print help information about the usage of the command.

Notice how the help command is completely auto-generated!
We will also get nice error output if mistakes are found in the input.

## Usage
### More help information
The names of commands may not be enough to describe what they do. Call `description`
to add extra help information to a command.
```rust
conso::args(|ctx| {
    ctx.command("greet")
        .description("Give the world a wonderful greeting")
        .run(|| {
            println!("Hello world!");
        });

    ctx.command("order")
        .description("Order something delicious")
        .run(|| {
            println!("I would like a boiled crab, please");
        });
});
```

### Subcommands
Subcommands can be added by calling `sub_commands`. This provides a new `ctx` that
can be used to add subcommands in the same way as normal commands.
```rust
conso::args(|ctx| {
    ctx.command("greet")
        .sub_commands(|ctx| {
            ctx.command("world")
                .run(|| {
                    println!("Hello world!");
                });

            ctx.command("you")
                .run(|| {
                    println!("Hello, you!");
                });
        });
});
```

`sub_commands` and `run` can be combined. In this case,
the `run` will happen if no valid sub commands were found (and as long as there are no more
arguments given, if there were an error will be emitted instead).
```rust
conso::args(|ctx| {
    ctx.command("greet")
        .sub_commands(|ctx| {
            ctx.command("crudely")
                .run(|| {
                    println!("Heyo world!");
                });
        })
        .run(|| {
            println!("Hello world!");
        });
});
```

Another way of acheiving the same thing is with `otherwise`.
```rust
conso::args(|ctx| {
    ctx.command("greet")
        .sub_commands(|ctx| {
            ctx.command("crudely")
                .run(|| {
                    println!("Heyo world!");
                });

            ctx.otherwise()
                .run(|| {
                    println!("Hello world!");
                });
        });
});
```

### Command groups
If there are a lot of commands and organization starts becoming necessary, it start becoming
necessary to bring out the big guns; good old functions!

```rust
fn greetings(ctx: &mut conso::Ctx) {
    ctx.command("crudely")
        // The help command is of course still automatically generated!
        .description("Crudely greet the world")
        .run(|| {
            println!("Heyo world!");
        });

    ctx.otherwise()
        .run(|| {
            println!("Hello world!");
        });
}

conso::args(|ctx| {
    ctx.command("greet")
        .sub_commands(greetings);

    // This command also gets the same subcommands, but we also add an extra
    // one called `dont`.
    ctx.command("maybegreet")
        .sub_commands(|ctx| {
            ctx.command("dont")
                .run(|| {});
            greetings(ctx);
        });
});
```

### Interactivity
Sometimes just command line arguments aren't enough. We might want to allow the user to input
commands in a loop. As it happens `user_loop` exists just for this purpose!

```rust
conso::user_loop(|ctx, control_flow| {
    ctx.command("greet")
        .run(|| {
            println!("Hello world!");
        });

    ctx.command("quit")
        .run(|| {
            control_flow.quit(());
        });
});
```

As opposed to `args`, the closure here takes an extra argument called `control_flow`, that
lets you tell conso when the loop should be finished using `quit`. This also allows data to be
passed to the caller. Other than that, it works exactly the same.

## Behind the scenes
The way this auto-generation works is a bit cheeky; and a hint can be found in the signature
of the `args` function:
```
pub fn args(handler: impl FnMut(&mut Ctx<'_, '_>)) {
    todo!();
}
```
Instead of taking an `FnOnce` closure like you might expect, it takes an `FnMut`. This lets
conso call it several times for different purposes. If the help command is called, or crono
wants to try and find suggested usages after an error has occured, conso will call this
function again, but in a special mode where nothing is really parsed and all `run` calls are
completely skipped.

What does this mean in practice? Nothing much, mostly you get a really simple way to define
commands, while also getting nice help information for free! The main thing to keep in mind is
not to run complex logic without being inside of a `run` call, since that logic probably should
not run when `help` is called.

The idea of the `control_flow` parameter inside of `user_loop` has a few big advantages;
one is that it allows `Ctx` to remain generic-less, which is a life-saver when grouping
commands together. It also allows nested user loops affect the control flows of parent user
loops easily. Originally the idea was to put a generic parameter on `Ctx` describing whether
it was a loop or not, and if it was a loop what return it had, but that was a pain so I'm much
happier with this approach. Originally `Command` and `DataCommand` were also going to be the same
type but with generics describing whether or not they had data attached, but that was scrapped
in favor of two types, with `Command` just being a thin wrapper over `DataCommand` instead.
