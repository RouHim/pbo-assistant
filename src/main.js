const {invoke} = window.__TAURI__.tauri;

let startTestButton;

async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

}

window.addEventListener("DOMContentLoaded", () => {
    // Register ui elements
    startTestButton = document.getElementById("startTestButton");


    // Register button actions
    startTestButton.addEventListener("click", startTestButtonClicked);
});

function startTestButtonClicked() {
    invoke("start_test", {});
}
