import { groupBy, renderState } from "../utils.js";
import { dataService } from "../dataService.js";

export const createStateBar = () => {
  const div = document.createElement("div");
  div.className = "stateBar";
  const length = 3;

  const a = Array.from({ length }, (_, index) => {
    if (index) div.appendChild(new Text(", "));
    div.insertAdjacentHTML("beforeEnd", renderState(index) + ": ");
    let v = document.createElement("var");
    v.textContent = 0;
    div.appendChild(v);
    return v;
  });

  dataService.addEventListener("update", (e) => {
    const { detail: list } = e;
    let m = groupBy(list, "state");
    for (let i = 0; i < 3; i++) {
      a[i].innerHTML = m[i] || 0;
    }
  });

  return div;
};
