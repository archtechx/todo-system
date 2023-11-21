function add(foo: any, bar: any): any { // todo@types
    return foo + bar;
}

function subtract(foo: any, bar: any): any { // todo@types add types
    return foo - bar;
}

function hello() {
    // console.log("Hello world!"); // todo000
}

function hello_world() {
    // todo00 add return typehint
    console.log("Hello world!");
}

function greet(name: any) {
    // todo0 add name typehint
    console.log(`Hello ${name}`);
}

function greet2(name: string) { // todo1 add return typehint
    console.log(`Hello ${name}`);
}

function echo(str: string) { // todo2 add return typehint
    console.log(str);
}
