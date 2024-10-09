const {invoke} = window.__TAURI__.core;
const {message} = window.__TAURI__.dialog;

const durationPerCoreInput = document.getElementById("durationPerCoreInput");
const coresToTestInput = document.getElementById("coresToTestInput");
const testMethodsLayout = document.getElementById("testMethodsLayout");
const startButton = document.getElementById("startButton");
const labelCoresToTest = document.getElementById("labelCoresToTest");

let timer;
let isTestRunning = false;
let physicalCoresCount = 0;
let appConfig = {};

window.addEventListener("DOMContentLoaded", () => {
    loadConfig().then(() => {
        loadTestMethods();
        loadCores();
    });

    startButton.addEventListener("click", () => onStartTestButtonClick());
});

function loadConfig() {
    return invoke("get_config").then((config) => {
        appConfig = JSON.parse(config);
        durationPerCoreInput.value = appConfig.test_duration_per_core;
        coresToTestInput.value = appConfig.cores_to_test;
    });
}

// Loads physical cores of the CPU
// Show them in the label: labelCoresToTest
function loadCores() {
    invoke("get_physical_cores").then((cores) => {
        physicalCoresCount = JSON.parse(cores) - 1;
        labelCoresToTest.innerText = `Physical cores to test (0 - ${physicalCoresCount})`;
    });
}

function clearSummaryLayout(innerHTML = "") {
    const summaryLayout = document.getElementById("summaryLayout");
    summaryLayout.innerHTML = innerHTML;
}

function onStartTestButtonClick() {
    if (isTestRunning) {
        clearSummaryLayout("Test stopped by user");
        onStopPressed();
    } else {
        clearSummaryLayout();
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

    // Build app config
    appConfig.test_duration_per_core = durationPerCore;
    appConfig.cores_to_test = coresToTest;
    appConfig.active_test_methods = testMethods;

    // Start the actual test
    invoke("start_test", {
        testMethods: testMethods,
        durationPerCore: durationPerCore,
        coresToTest: coresToTest,
        appConfig: JSON.stringify(appConfig),
    }).then((_) => {
        isTestRunning = true;
        startButton.innerText = "Stop";
        startStatusPolling();
    }).catch(async (errorMsg) => {
        await message(errorMsg, {title: 'Error', kind: 'error'});
    });
}

function onStopPressed() {
    invoke("stop_test").then(() => {
        stopTest();
    });
}

function stopTest() {
    clearInterval(timer);
    isTestRunning = false;
    startButton.innerText = "Start";
    updateTestStatus();
}

function updateCpuStatus(cpuTestStatus) {
    // Find div layout for the current core
    let cpuLayout = document.getElementById(`cpu${cpuTestStatus.core_id}`);

    // If it doesn't exist, create it
    if (!cpuLayout) {
        createCpuStatusLayout(cpuTestStatus, cpuLayout);
    } else {
        // The CPU layout already exists, just update the according values
        updateCpuStatusLayout(cpuTestStatus, cpuLayout);
    }
}

// Updates the offset of the core by the given delta
function addValueToOffset(coreId, toAdd) {
    const offsetInput = document.getElementById(`offset${coreId}`);
    let nextValue = parseInt(offsetInput.value) + toAdd;

    // Limit to -30 to 30
    nextValue = Math.min(30, Math.max(-30, nextValue));

    saveOffset(coreId, nextValue, offsetInput);
}

function saveOffset(coreId, newValue, offsetInput) {
    invoke("set_offset", {coreId: coreId, offset: newValue});
    offsetInput.value = newValue;
    appConfig.offset_per_core[coreId] = newValue;
}

function createCpuStatusLayout(cpuTestStatus, cpuLayout) {
    const cpusLayout = document.getElementById("cpusLayout");
    const div = document.createElement("div");
    div.id = `cpu${cpuTestStatus.core_id}`;
    div.className = "cpuLayout";
    cpusLayout.appendChild(div);
    cpuLayout = div;

    // Core id as span
    let coreId = document.createElement("span");
    coreId.innerText = `Core ${cpuTestStatus.core_id}`;
    coreId.className = "coreId";
    cpuLayout.appendChild(coreId);

    // "Offset" static text
    cpuLayout.appendChild(document.createElement("br"));
    cpuLayout.appendChild(document.createTextNode("Offset"));
    cpuLayout.appendChild(document.createElement("br"));

    // Create a container div for the buttons
    const buttonContainer = document.createElement("div");
    buttonContainer.className = "buttonContainer";
    cpuLayout.appendChild(buttonContainer);

    // "-" Button, that reduces the offset by 1
    const offsetMinusButton = document.createElement("button");
    offsetMinusButton.innerText = "-";
    offsetMinusButton.onclick = () => addValueToOffset(cpuTestStatus.core_id, -1);
    buttonContainer.appendChild(offsetMinusButton);

    const offsetInput = document.createElement("input");
    offsetInput.type = "number";
    offsetInput.id = `offset${cpuTestStatus.core_id}`;
    offsetInput.value = 0;
    offsetInput.min = -30;
    offsetInput.max = 30;
    buttonContainer.appendChild(offsetInput);
    // Add on focus lost listener, also set the offset value to the app config
    offsetInput.addEventListener("focusout", () => {
        saveOffset(cpuTestStatus.core_id, parseInt(offsetInput.value), offsetInput);
    });

    // Load the offset value from the app config from property "offset_per_core"
    // and set the value to the input field
    const offset = appConfig.offset_per_core[cpuTestStatus.core_id];
    if (offset) {
        offsetInput.value = offset;
    }

    // "+" Button, that increases the offset by 1
    const offsetPlusButton = document.createElement("button");
    offsetPlusButton.innerText = "+";
    offsetPlusButton.onclick = () => addValueToOffset(cpuTestStatus.core_id, 1);
    buttonContainer.appendChild(offsetPlusButton);

    // The clock speed as text eg "3600 MHz"
    cpuLayout.appendChild(document.createElement("br"));
    const maxClockTextNode = document.createElement("span");
    maxClockTextNode.id = `${cpuTestStatus.core_id}Clock`;
    maxClockTextNode.innerText = `${cpuTestStatus.max_clock} MHz`;
    maxClockTextNode.title = "Maximum Clock of the Core";
    cpuLayout.appendChild(maxClockTextNode);

    // The test methods in one line as dedicated spans
    const methods = cpuTestStatus.method_response;
    // Create div that contains the method status
    const methodStatusLayout = document.createElement("div");
    methodStatusLayout.className = "methodStatusLayout";
    for (const method in methods) {
        const methodStatusTextNode = document.createElement("span");
        methodStatusTextNode.id = `${cpuTestStatus.core_id}${method}`;
        methodStatusTextNode.className = "methodStatus";
        methodStatusLayout.appendChild(methodStatusTextNode);
    }
    cpuLayout.appendChild(methodStatusLayout);

    // The Progress bar showing the time left for the current test method
    // Hidden at the beginning
    const progressBar = document.createElement("progress");
    progressBar.id = `${cpuTestStatus.core_id}ProgressBar`;
    progressBar.className = "progressBar";
    progressBar.max = 100;
    progressBar.value = 0;
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

    // Update clock speed
    let maxClockTextNode = document.getElementById(`${cpuTestStatus.core_id}Clock`);
    setValueAnimated(maxClockTextNode, cpuTestStatus.max_clock, " MHz");

    // Update progress bar
    const progressBar = document.getElementById(`${cpuTestStatus.core_id}ProgressBar`);
    progressBar.style.display = isAnyMethodTesting ? "block" : "none";
    if (isAnyMethodTesting) {
        const currentMethodInTesting = Object.values(methods).find((method) => method.state === "Testing");
        progressBar.max = currentMethodInTesting.total_secs;
        progressBar.value = currentMethodInTesting.current_secs;
    }

    // Update Test method status
    for (const method in methods) {
        const methodStatusTextNode = document.getElementById(`${cpuTestStatus.core_id}${method}`);
        methodStatusTextNode.innerText = `${method}`;
        switch (methods[method].state) {
            case "Idle":
                methodStatusTextNode.style.borderColor = "transparent";
                break;
            case "Testing":
                methodStatusTextNode.style.borderColor = "var(--selection)";
                break;
            case "Success":
                methodStatusTextNode.style.borderColor = "#00a000";
                break;
            case "Failed":
                methodStatusTextNode.style.borderColor = "#ff0000";
                break;
        }
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

function setValueAnimated(textInput, nextValue, suffix) {
    let currentValue = parseInt(textInput.innerText);

    // If currentValue == nextValue, do nothing
    if (currentValue === nextValue) {
        return;
    }

    const diff = nextValue - currentValue;
    const step = diff / 10;
    let counter = 0;
    const interval = setInterval(() => {
        currentValue += step;
        textInput.innerText = Math.round(currentValue) + suffix;
        counter++;
        if (counter >= 10) {
            clearInterval(interval);
            textInput.innerText = nextValue + suffix;
        }
    }, 50);
}

function startStatusPolling() {
    timer = setInterval(() => updateTestStatus(), 1000);
}

function isWholeTestDone(testStatus) {
    // Collect all test method results
    let allMethods = testStatus.flatMap(cpuTestStatus => Object.values(cpuTestStatus.method_response));

    // Check if all states are either success or failed
    return allMethods.every(method => method.state === "Success" || method.state === "Failed");
}

function updateTestStatus() {
    invoke("get_test_status").then((results) => {
        const testStatus = JSON.parse(results);

        testStatus.forEach((cpuTestStatus) => {
            updateCpuStatus(cpuTestStatus);
        });

        if (isWholeTestDone(testStatus)) {
            stopTest();
            showSummary(testStatus);
        }
    });
    // .catch((error) => {
    // console.error("Error while getting test status: " + error);
    // });
}

// Shows a summary of the test results
// If all cores passed the test, it will show a success message
// If any core failed the test, it will show a list of the failed cores
function showSummary(testStatus) {
    const summaryLayout = document.getElementById("summaryLayout");
    summaryLayout.innerHTML = "";

    const failedCores = testStatus.filter((cpuTestStatus) => {
        const methods = cpuTestStatus.method_response;
        return Object.values(methods).some((method) => method.state === "Failed");
    });

    if (failedCores.length > 0) {
        const div = document.createElement("div");
        div.innerText = "Failed cores: " + failedCores
            .map((cpuTestStatus) => cpuTestStatus.core_id)
            .join(", ");
        summaryLayout.appendChild(div);
    } else {
        const div = document.createElement("div");
        div.innerText = "All cores passed the test";
        summaryLayout.appendChild(div);
    }
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

    // Check if the test method is present in the app config "active_test_methods",
    let isActive = appConfig.active_test_methods.includes(testMethodName);
    checkbox.checked = isActive;

    div.appendChild(label);
    return div;
}
