import { logger } from "@@shared/logger";

type Handler<T = any> = (payload: T) => Promise<void> | void;

class EventBus {
    private handlers = new Map<string, Handler[]>();

    on<T = any>(event: string, handler: Handler<T>): void {
        const list = this.handlers.get(event) ?? [];
        list.push(handler as Handler);
        this.handlers.set(event, list);
    }

    /** Fire-and-forget, meant to be called only after the source transaction commits. */
    emit<T = any>(event: string, payload: T): void {
        const list = this.handlers.get(event) ?? [];
        for (const handler of list) {
            Promise.resolve()
                .then(() => handler(payload))
                .catch((err) => {
                    logger.error({ err, event }, "event handler failed");
                });
        }
    }
}

export const bus = new EventBus();
export default bus;
