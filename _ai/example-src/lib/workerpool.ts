// This file manages a pool of web workers
//
// NOTE: This WorkerPool uses traditional postMessage-based communication.
// For Comlink-wrapped workers (see src/fractals/workers/fractal.worker.ts),
// a different pool management approach may be needed in Story 3.
// Comlink handles message passing via RPC, so the custom message protocol
// below may not be necessary for Comlink workers.

// Define message types for better type safety
export interface WorkerMessage {
  taskId?: number;
  type: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  [key: string]: any; // Allow additional properties
}

export interface WorkerPoolOptions {
  workerScript: string;
  maxWorkers?: number;
  onError?: (error: ErrorEvent, workerId: number) => void;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export interface Task<TData extends object = object, TResult = any> {
  id: number;
  data: TData;
  resolve: (result: TResult) => void;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  reject: (error: any) => void;
}

export class WorkerPool {
  private workers: Worker[] = [];
  private availableWorkers: number[] = [];
  private taskQueue: Task[] = [];
  private activeTasks: Map<number, Task> = new Map(); // Track active tasks by ID
  private taskIdCounter = 0;
  private isInitialized = false;
  private initPromise: Promise<void>;
  private initResolve!: () => void;
  private workerReadyCount = 0;

  constructor(private options: WorkerPoolOptions) {
    // Create a promise that resolves when all workers are initialized
    this.initPromise = new Promise<void>((resolve) => {
      this.initResolve = resolve;
    });

    // Determine the number of workers to create
    const maxWorkers = options.maxWorkers || this.getOptimalWorkerCount();

    // Create the workers
    for (let i = 0; i < maxWorkers; i++) {
      this.createWorker(i);
    }
  }

  // Wait for all workers to be initialized
  public async waitForInit(): Promise<void> {
    return this.initPromise;
  }

  // Get the optimal number of workers based on CPU cores
  private getOptimalWorkerCount(): number {
    // Use navigator.hardwareConcurrency if available, otherwise default to 4
    const cpuCount = navigator.hardwareConcurrency || 4;

    // Use 75% of available cores, but at least 2 and at most 16
    return Math.max(2, Math.min(16, Math.floor(cpuCount * 0.75)));
  }

  // Create a new worker
  private createWorker(id: number): void {
    const worker = new Worker(this.options.workerScript, { type: "module" });

    worker.onmessage = (event) => this.handleWorkerMessage(event, id);
    worker.onerror = (error) => this.handleWorkerError(error, id);

    this.workers[id] = worker;
  }

  // Handle messages from workers
  private handleWorkerMessage(event: MessageEvent, workerId: number): void {
    const data = event.data as WorkerMessage;

    // console.log(`Received ${data.type} message from worker ${workerId} with task id ${data.taskId}`); // Log the message for debugging purpose

    // Handle worker ready message
    if (data.type === "ready") {
      this.workerReadyCount++;
      this.availableWorkers.push(workerId);

      // If all workers are ready, resolve the init promise
      if (this.workerReadyCount === this.workers.length) {
        this.isInitialized = true;
        this.initResolve();
        this.processQueue();
      }
      return;
    }

    const taskId = data.taskId;
    if (taskId !== undefined) {
      // Find the task in our active tasks map
      const task = this.activeTasks.get(taskId);

      if (task) {
        // Remove the task from active tasks
        this.activeTasks.delete(taskId);

        // Resolve the task's promise with the result data
        // Remove the taskId from the result to clean it up
        const result = { ...data };
        delete result.taskId;
        task.resolve(result);
      }

      // Mark the worker as available
      this.availableWorkers.push(workerId);

      // Process the next task in the queue
      this.processQueue();
    }
  }

  // Handle worker errors
  private handleWorkerError(error: ErrorEvent, workerId: number): void {
    if (this.options.onError) {
      this.options.onError(error, workerId);
    }

    // Mark the worker as available despite the error
    this.availableWorkers.push(workerId);
    this.processQueue();
  }

  // Execute a task on an available worker
  public async execute<TData extends object, TResult>(data: TData): Promise<TResult> {
    // Create a new task
    const taskId = this.taskIdCounter++;

    // Create a promise that will be resolved when the task completes
    const taskPromise = new Promise<TResult>((resolve, reject) => {
      this.taskQueue.push({
        id: taskId,
        data,
        resolve,
        reject,
      });
    });

    // If we're initialized, process the queue
    if (this.isInitialized) {
      this.processQueue();
    }

    return taskPromise;
  }

  // Process the next task in the queue
  private processQueue(): void {
    // If there are no tasks or no available workers, return
    if (this.taskQueue.length === 0 || this.availableWorkers.length === 0) {
      return;
    }

    // Get the next task and worker
    const task = this.taskQueue.shift()!;
    const workerId = this.availableWorkers.shift()!;
    const worker = this.workers[workerId];

    // Store the task in our active tasks map
    this.activeTasks.set(task.id, task);

    // Send the task to the worker
    worker.postMessage({
      ...task.data,
      taskId: task.id,
    });
  }

  // Send a message to all workers
  public broadcast(message: object): void {
    for (const worker of this.workers) {
      worker.postMessage(message);
    }
  }

  // Terminate all workers
  public terminate(): void {
    for (const worker of this.workers) {
      worker.terminate();
    }

    this.workers = [];
    this.availableWorkers = [];
    this.taskQueue = [];
    this.isInitialized = false;
  }

  // Get the number of workers
  public getWorkerCount(): number {
    return this.workers.length;
  }
}
