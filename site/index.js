const js = import("./rustmigemo.js");
let migemo = null;
js.then(js => {
    fetch('migemo-compact-dict')
        .then(function (response) {
            return response.arrayBuffer()
        }).then(function (buffer) {
            let array = new Uint8Array(buffer);
            migemo = js.Migemo.new(array)
        })
});

document.getElementById("query").addEventListener("input", function(e) {
    let result = migemo.query(e.target.value);
    document.getElementById("result").textContent = result;
})

