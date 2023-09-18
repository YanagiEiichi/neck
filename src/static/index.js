import { dataService } from "./dataService.js";
import { createLiveTime } from "./liveTime.js";
import { renderState } from "./utils.js";

const createTable = () => {
  const table = document.createElement("table");

  const createRow = (data) => {
    const row = table.insertRow();
    row.dataset.id = data.id;
    const cells = Array.from({ length: 6 }, () => row.insertCell());
    row.update = (data) => {
      cells[0].textContent = data.id;
      cells[1].textContent = data.proto;
      cells[2].innerHTML = renderState(data.state);
      cells[3].textContent = data.host;
      cells[4].textContent = data.from;
      if (cells[5].timestampe !== data.timestamp) {
        cells[5].timestampe = data.timestamp;
        cells[5].innerHTML = "";
        cells[5].appendChild(createLiveTime(data.timestamp));
      }
    };
    row.update(data);
    return row;
  };

  const updateTableData = (list) => {
    const m = new Map(list.map((i) => [String(i.id), i]));
    const { rows } = table;
    for (let i = 1; i < rows.length; i++) {
      const row = rows[i];
      const data = m.get(row.dataset.id);
      if (data) {
        m.delete(row.dataset.id);
        row.update(data);
      } else {
        row.remove();
        i--;
      }
    }
    for (const data of m.values()) {
      createRow(data);
    }
  };

  dataService.addEventListener("update", (e) => {
    updateTableData(e.detail);
  });

  const row = table.insertRow();
  row.insertCell().textContent = "ID";
  row.insertCell().textContent = "Type";
  row.insertCell().textContent = "State";
  row.insertCell().textContent = "Host";
  row.insertCell().textContent = "From";
  row.insertCell().textContent = "Uptime";

  return table;
};

const createActivityState = () => {
  const div = document.createElement("div");
  div.className = "activityState";

  dataService.addEventListener("active", () => {
    div.title = "Online";
  });
  dataService.addEventListener("inactive", () => {
    div.title = "Offline";
  });

  return div;
};

const createStateBar = () => {
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
    const map = Array(length).fill(0);
    list.forEach((i) => map[i.state]++);
    map.forEach((v, i) => {
      a[i].innerHTML = v;
    });
  });

  return div;
};

const createHeader = () => {
  const header = document.createElement("header");

  const img = new Image();
  img.src = "./neck.png";
  img.alt = "logo";
  header.appendChild(img);
  header.appendChild(createStateBar());
  header.appendChild(createActivityState());
  return header;
};

const main = async () => {
  document.body.appendChild(createHeader());

  const main = document.createElement("main");
  const tip = createTableTip();

  dataService.addEventListener("active", () => {
    document.body.classList.add("living");
  });

  dataService.addEventListener("inactive", () => {
    document.body.classList.remove("living");
  });

  dataService.addEventListener("update", (e) => {
    let { detail } = e;
    tip.update("Empty");
    if (detail.length) {
      tip.update();
    } else {
      tip.update("Empty");
    }
  });

  main.appendChild(createTable());
  main.appendChild(tip);
  document.body.appendChild(main);
};

const createTableTip = () => {
  const div = document.createElement("div");
  div.className = "tableTip";
  const empty = new Comment(" TableTip ");
  const update = (html) => {
    if (html) {
      div.innerHTML = html;
      if (empty.parentNode) empty.replaceWith(div);
    } else {
      if (div.parentNode) div.replaceWith(empty);
    }
  };
  div.update = update;
  empty.update = update;
  return empty;
};

document.body ? main() : addEventListener("load", main);
