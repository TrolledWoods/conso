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

### Parsing data
Sometimes we might want a command to be able to recieve data. This can be acheived
by calling the `arg` function, specifying the type of data we want to recieve.
After doing so, our `run` method closure will gain a parameter containing the argument that was passed.

```rust
conso::args(|ctx| {
    ctx.command("echo")
        .arg::<String>()
        .run(|message| {
            println!("{}", message);
        });
});
```

If you want several arguments, you can request a tuple of (almost) any size from the `arg` function.

```rust
conso::args(|ctx| {
    ctx.command("echo1")
        .arg::<(String, String)>()
        .run(|(message1, message2)| {
            println!("{}, then {}", message1, message2);
        });
});
```
You can also call the `arg` function several times in succession, but it's more confusing so I will leave that out.

For some arguments you may want to make sure they are within a certain bound. For that there is the `constrained_arg` function!
It takes in arguments describing the constraints, in this case saying that we want two numbers between 0 and 100.
```rust
conso::args(|ctx| {
    ctx.command("multiply")
        .constrained_arg((0..100, 0..100))
        .run(|(a, b)| {
            println!("{} * {} = {}", a, b, a * b);
        });
});
```

One funny, or maybe scary thing about the `command` function we have been using up until now, is that it actually takes in a constraint
exactly like `constrained_arg`! If the constraint given is fulfilled, then the command is ran. This means we can
make crazy commands like this too;
```rust
conso::args(|ctx| {
    ctx.command(0..10)
        .run(|| {
            println!("The number you entered was between 0 and 10");
        });

    ctx.command(100..110)
        .run(|| {
            println!("The number you entered was between 100 and 110");
        });

    // `otherwise` is actually just a wrapper over `ctx.command(())`, since
    // `()` is a constraint that always passes.
    ctx.otherwise()
        .run(|| {
            println!("You didn't enter a number");
        });
});
```

We can also get the actual value of the entered numbers by using `data_command` instead.
```rust
conso::args(|ctx| {
    ctx.data_command(0..10)
        .run(|number| {
            println!("The number you entered was {}", number);
        });
});
```

### Command groups
If there are a lot of commands and organization starts becoming necessary, we may have
to bring out the big guns; good old functions!

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

### Aliases
Some commands are so common that you might want a shorter name for them. Since command names are really
just constraints, we can use the `either` function to combine two constraints!

```rust
conso::user_loop(|ctx, control_flow| {
    ctx.command(conso::either("q", "quit"))
        .run(|| {
            control_flow.quit(());
        });
});
```

### Behind the scenes
The way the help auto-generation works is a bit cheeky; and a hint can be found in the signature
of the `args` function:
```rust
pub fn args(handler: impl FnMut(&mut conso::Ctx<'_, '_>)) {
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
