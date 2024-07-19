const {invoke} = window.__TAURI__.core;

const durationPerCoreInput = document.getElementById("durationPerCoreInput");
const coresToTestInput = document.getElementById("coresToTestInput");
const testMethodsLayout = document.getElementById("testMethodsLayout");
const startButton = document.getElementById("startButton");

let timer;
let isTestRunning = false;

window.addEventListener("DOMContentLoaded", () => {
    loadTestMethods();

    startButton.addEventListener("click", () => onStartTestButtonClick());
});

function onStartTestButtonClick() {
    if (isTestRunning) {
        stopTest();
    } else {
        startTest();
    }
}

function startTest() {
    const testMethods = [];
    document.querySelectorAll('.testMethod input[type=checkbox]')
        .forEach((checkbox) => {
            if (checkbox.checked) {
                testMethods.push(checkbox.value);
            }
        });
    const durationPerCore = durationPerCoreInput.value;
    const coresToTest = coresToTestInput.value;

    // Clear cpusLayout
    const cpusLayout = document.getElementById("cpusLayout");
    cpusLayout.innerHTML = "";

    // Start the actual test
    invoke("start_test", {
        testMethods: testMethods,
        durationPerCore: durationPerCore,
        coresToTest: coresToTest
    }).then((result) => {
        isTestRunning = true;
        startButton.innerText = "Stop";
        startStatusPolling();
    });
}

function stopTest() {
    invoke("stop_test").then(() => {
        clearInterval(timer);
        isTestRunning = false;
        startButton.innerText = "Start";
    });
}

function updateCpuStatus(cpuTestStatus) {
    // Find div layout
    let cpuLayout = document.getElementById(`cpu${cpuTestStatus.core_id}`);

    // If it doesn't exist, create it
    if (!cpuLayout) {
        createCpuStatusLayout(cpuTestStatus, cpuLayout);
    } else {
        // The CPU layout already exists, just update the according values
        updateCpuStatusLayout(cpuTestStatus, cpuLayout);
    }

}

function createCpuStatusLayout(cpuTestStatus, cpuLayout) {
    const cpusLayout = document.getElementById("cpusLayout");
    const div = document.createElement("div");
    div.id = `cpu${cpuTestStatus.core_id}`;
    div.className = "cpuLayout";
    cpusLayout.appendChild(div);
    cpuLayout = div;

    // Core id
    cpuLayout.appendChild(document.createTextNode(`# ${cpuTestStatus.core_id}`));

    // Create and set current test method status
    const methodsDiv = document.createElement("div");
    methodsDiv.id = `${cpuTestStatus.core_id}MethodLayout`;
    const methods = cpuTestStatus.method_response;
    for (const method in methods) {
        const methodDiv = document.createElement("div");
        const methodStatus = methods[method];
        methodDiv.id = `${cpuTestStatus.core_id}${method}Status`;
        methodDiv.innerText = `${method}: ${methodStatus.state}`;
        methodsDiv.appendChild(methodDiv);
    }
    cpuLayout.appendChild(methodsDiv);

    // Create and set clock speed
    const clockDiv = document.createElement("div");
    clockDiv.id = `${cpuTestStatus.core_id}Clock`;
    clockDiv.innerText = `Max. Clock: ${cpuTestStatus.max_clock} MHz`;
    cpuLayout.appendChild(clockDiv);

    // Create and set progress
    const progressDiv = document.createElement("div");
    progressDiv.id = `${cpuTestStatus.core_id}Progress`;
    progressDiv.style.display = "none";
    cpuLayout.appendChild(progressDiv);

    // Create and set progress bar
    const progressBar = document.createElement("progress");
    progressBar.id = `${cpuTestStatus.core_id}ProgressBar`;
    progressBar.style.display = "none";
    cpuLayout.appendChild(progressBar);
}

function updateCpuStatusLayout(cpuTestStatus, cpuLayout) {
    const methods = cpuTestStatus.method_response;

    // Determine cpu states
    let isAllMethodsIdle = Object.values(methods).every((method) => method.state === "Idle");
    let isAnyMethodTesting = Object.values(methods).some((method) => method.state === "Testing");
    let isAllMethodsSuccess = Object.values(methods).every((method) => method.state === "Success");
    let isAnyMethodFailed = Object.values(methods).some((method) => method.state === "Failed");
    let isAnyIdleAndAnySuccess = Object.values(methods).some((method) => method.state === "Idle") && Object.values(methods).some((method) => method.state === "Success");

    // Update current status
    for (const method in methods) {
        let methodStatusTextNode = document.getElementById(`${cpuTestStatus.core_id}${method}Status`);
        const methodStatus = methods[method];
        methodStatusTextNode.innerText = `${method}: ${methodStatus.state}`;
    }

    // Update clock speed
    const maxClockTextNode = document.getElementById(`${cpuTestStatus.core_id}Clock`);
    maxClockTextNode.innerText = `Max. Clock: ${cpuTestStatus.max_clock} MHz`;

    // Update the progress
    let currentMethodInTesting = Object.values(methods).find((method) => method.state === "Testing");

    // Update progress text
    const progressTextNode = document.getElementById(`${cpuTestStatus.core_id}Progress`);
    progressTextNode.style.display = isAnyMethodTesting ? "block" : "none";
    if (isAnyMethodTesting) {
        progressTextNode.innerText = `Progress: ${currentMethodInTesting.current_secs}/${currentMethodInTesting.total_secs}`;
    }

    // Update progress bar
    const progressBar = document.getElementById(`${cpuTestStatus.core_id}ProgressBar`);
    progressBar.style.display = isAnyMethodTesting ? "block" : "none";
    if (isAnyMethodTesting) {
        progressBar.max = currentMethodInTesting.total_secs;
        progressBar.value = currentMethodInTesting.current_secs;
    }

    // Update verification status
    // Set style classes of cpu div accordingly the current state ( Idle,  Testing, Success, Failed,):
    // if:
    // - All methods in idle state -> set idle style class
    // - Any method is in testing state -> set testing style class
    // - All methods in success state -> set success style class
    // - Any method in failed state -> set failed style class
    let className = "cpuLayout";
    if (isAllMethodsIdle) {
        className += " cpu-idle";
    } else if (isAnyMethodTesting || isAnyIdleAndAnySuccess) {
        className += " cpu-testing";
    } else if (isAllMethodsSuccess) {
        className += " cpu-success";
    } else if (isAnyMethodFailed) {
        className += " cpu-failed";
    }
    cpuLayout.className = className;
}

function startStatusPolling() {
    timer = setInterval(() => updateTestStatus(), 1000);
}

function updateTestStatus() {
    console.log("Updating test status...");
    invoke("get_test_status").then((results) => {
        console.log("Got test status");
        const testStatus = JSON.parse(results);
        testStatus.forEach((cpuTestStatus) => {
            updateCpuStatus(cpuTestStatus);
        });
    }).catch((error) => {
        console.error("Error while getting test status: " + error);
    });
}

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
