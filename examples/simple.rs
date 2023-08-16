fn main() {
    conso::user_loop(|ctx, control_flow| {
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
                    .user_loop(|ctx, control_flow| {
                        ctx.command("hi").run(|| println!("Hi!"));
                        ctx.command("quit").run(|| control_flow.quit(()));
                    });
            })
            .run(|| {
                println!("Hello, world!");
            });

        ctx.command("multiply")
            .constrained_arg((0..100, 0..100))
            .run(|(a, b)| {
                println!("{} * {} = {}", a, b, a * b);
            });
        
        ctx.command("quit")
            .run(|| control_flow.quit(()));
    });
}
