import readline from "node:readline";

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

rl.on("line", (line) => {
    const [type, method, payload] = line.trim().split("\t");
    switch (type) {
        case "request":
            console.log(`response\t${method}\t${payload}`);
            break;
        default:
            console.log(`error\t${method}\t"not implemented yet: ${type}"`);
    }
})
    .on("close", () => {
    process.exit(0);
})
