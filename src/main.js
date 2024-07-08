const {invoke} = window.__TAURI__.core;

const durationPerCoreInput = document.getElementById("durationPerCoreInput");
const coresTotTestInput = document.getElementById("coresTotTestInput");
const testMethodsLayout = document.getElementById("testMethodsLayout");
const startButton = document.getElementById("startButton");

function startTest() {
    const testMethods = [];
    document.querySelectorAll('.testMethod input[type=checkbox]').forEach((checkbox) => {
        if (checkbox.checked) {
            testMethods.push(checkbox.value);
        }
    });

    const durationPerCore = durationPerCoreInput.value;
    const coresTotTest = coresTotTestInput.value;

    invoke("start_test", {
        testMethods: testMethods,
        durationPerCore: durationPerCore,
        coresTotTest: coresTotTest
    });
}

window.addEventListener("DOMContentLoaded", () => {
    loadTestMethods();

    startButton.addEventListener("click", () => {
        startTest();

    });
});

function loadTestMethods() {
    invoke("get_test_methods").then((methods) => {
        JSON.parse(methods)
            .forEach((method) => {
                const div = createTestMethodCheckbox(method);
                testMethodsLayout.appendChild(div);
            });
    });
}

function createTestMethodCheckbox(testMethodName) {
    const div = document.createElement("div");
    div.id = testMethodName + "Layout";
    div.className = "testMethod";

    // Add checkbox
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.id = testMethodName + "Checkbox";
    checkbox.value = testMethodName;
    checkbox.checked = true
    div.appendChild(checkbox);

    // Create Label for Checkbox
    const label = document.createElement("label");
    label.htmlFor = testMethodName + "Checkbox";
    label.appendChild(document.createTextNode(testMethodName));

    div.appendChild(label);
    return div;
}
