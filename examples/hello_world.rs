fn main() {
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
}
