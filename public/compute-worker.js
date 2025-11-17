// Worker wrapper script that initializes the fractalwonder-compute WASM module
// This script loads the wasm-bindgen generated code and calls init_worker()

// Load the wasm-bindgen generated JavaScript
importScripts('./fractalwonder-compute.js');

// Load WASM synchronously using XMLHttpRequest
// This ensures the worker is fully initialized before receiving any messages
try {
    // Fetch WASM bytes synchronously
    const xhr = new XMLHttpRequest();
    xhr.open('GET', './fractalwonder-compute_bg.wasm', false); // false = synchronous
    xhr.responseType = 'arraybuffer';
    xhr.send();

    if (xhr.status !== 200) {
        throw new Error(`Failed to load WASM: HTTP ${xhr.status}`);
    }

    // Initialize WASM synchronously
    const wasmBytes = xhr.response;
    const wasmModule = new WebAssembly.Module(wasmBytes);
    wasm_bindgen.initSync(wasmModule);

    // Call init_worker() to set up the message handler
    // This function is exported from worker.rs via #[wasm_bindgen]
    wasm_bindgen.init_worker();

    console.log('Worker initialized successfully');
} catch (err) {
    console.error('Worker initialization failed:', err);
    throw err;
}
