function setResult(value) {
    Deno.core.opSync('op_set_result', value.toString());
}

function print(value) {
    Deno.core.print(value.toString() + "\n");
}

function printerr(value) {
    Deno.core.print(value.toString() + "\n", true);
}
