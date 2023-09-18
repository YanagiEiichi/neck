import { dataService } from "./dataService.js";
import { groupBy } from "./utils.js";
import { createHeader } from "./components/header.js";
import { createTableTip } from "./components/tableTip.js";
import { createMainTable } from "./components/mainTable.js";

const main = async () => {
  document.body.appendChild(createHeader());

  const main = document.createElement("main");
  const tip = createTableTip();

  dataService.addEventListener("inactive", () => {
    document.title = "Offline · Neck";
  });

  dataService.addEventListener("update", (e) => {
    let { detail: list } = e;

    let m = groupBy(list, "state");
    m.length = 3;
    let a = Array.from(m, (v) => v || 0);
    document.title = `(${a}) · Neck`;

    tip.update("Empty");
    if (list.length) {
      tip.update();
    } else {
      tip.update("Empty");
    }
  });

  main.appendChild(createMainTable());
  main.appendChild(tip);
  document.body.appendChild(main);
};

document.body ? main() : addEventListener("load", main);
