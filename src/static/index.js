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
  dataService.addEventListener("inactive", (e) => {
    updateTableData([]);
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

  dataService.addEventListener("active", (e) => {
    div.title = "Online";
  });
  dataService.addEventListener("inactive", (e) => {
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

  const update = (list) => {
    const map = Array(length).fill(0);
    list.forEach((i) => map[i.state]++);
    map.forEach((v, i) => {
      a[i].innerHTML = v;
    });
  };

  dataService.addEventListener("inactive", (e) => {
    update([]);
  });
  dataService.addEventListener("update", (e) => {
    update(e.detail);
  });

  return div;
};

const createHeader = () => {
  const header = document.createElement("header");

  header.appendChild(createStateBar());
  header.appendChild(createActivityState());
  return header;
};

const main = async () => {
  dataService.addEventListener("active", (e) => {
    document.body.classList.add("living");
  });

  dataService.addEventListener("inactive", (e) => {
    document.body.classList.remove("living");
  });

  document.body.appendChild(createHeader());
  const main = document.createElement('main');
  main.appendChild(createTable());
  document.body.appendChild(main);
};

document.body ? main() : addEventListener("load", main);
