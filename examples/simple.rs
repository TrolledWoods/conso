fn main() {
    conso::user_loop(|ctx| {
        ctx.command("print")
            .description("Prints a hello world message")
            .sub_commands(|ctx| {
                ctx.command("placehold")
                    .description("Prints placeholder text")
                    .run(|| {
                        println!("Lorem ipsum dolor sit amet");
                    });
                
                ctx.command("repeat")
                    .description("Prints on repeat until you stop it")
                    .user_loop(|ctx| {
                        ctx.command("hi").run(|| println!("Hi!"));
                        ctx.command("quit").run_with(|ctx| ctx.quit(()));
                    });
            })
            .run(|| {
                println!("Hello, world!");
            });
        
        ctx.command("quit")
            .run_with(|ctx| ctx.quit(()));
    });
}
