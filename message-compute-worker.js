import init, { init_message_worker } from './worker-pkg/fractalwonder_compute.js';

async function run() {
    await init();
    init_message_worker();
}

run();
