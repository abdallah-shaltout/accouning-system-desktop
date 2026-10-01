// No @types/node-cron package is installed; this is the minimal ambient shape this project uses.
declare module "node-cron" {
    export interface ScheduledTask {
        start(): void;
        stop(): void;
    }

    export function schedule(expression: string, fn: () => void, options?: Record<string, unknown>): ScheduledTask;

    const cron: {
        schedule: typeof schedule;
    };

    export default cron;
}
