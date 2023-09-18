import { singleFlight } from "./utils.js";

class DataService {
  constructor() {
    this.#keeper();
  }

  #et = new EventTarget();

  addEventListener(...args) {
    return this.#et.addEventListener(...args);
  }
  removeEventListener(...args) {
    return this.#et.removeEventListener(...args);
  }

  async #keeper() {
    for (;;) {
      try {
        this.#update();
        await this.#initiateEventSource();
      } catch (e) {
        void e;
      }
      await new Promise((f) => setTimeout(f, 1000));
    }
  }

  #update = singleFlight(async () => {
    return fetch("api/sessions")
      .then((res) => res.json())
      .then(
        (list) => {
          this.#et.dispatchEvent(new CustomEvent("update", { detail: list }));
        },
        () => {
          this.#et.dispatchEvent(new CustomEvent("update", { detail: [] }));
        }
      );
  });

  #initiateEventSource = singleFlight(
    () =>
      new Promise((_, reject) => {
        let es = new EventSource("api/events");
        es.addEventListener("init", () => {
          this.#et.dispatchEvent(new CustomEvent("active"));
        });
        es.addEventListener("update", this.#update);
        es.addEventListener("error", () => {
          this.#et.dispatchEvent(new CustomEvent("inactive"));
          es.close();
          reject();
        });
      })
  );
}

export const dataService = new DataService();
