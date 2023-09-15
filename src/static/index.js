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

const createHeader = () => {
  const header = document.createElement("header");
  const length = 3;

  const a = Array.from({ length }, (_, index) => {
    if (index) header.appendChild(new Text(", "));
    header.insertAdjacentHTML("beforeEnd", renderState(index) + ": ");
    let v = document.createElement("var");
    v.textContent = 0;
    header.appendChild(v);
    return v;
  });

  const activityState = document.createElement("div");
  activityState.className = "activityState";
  header.appendChild(activityState);

  const update = (list) => {
    const map = Array(length).fill(0);
    list.forEach((i) => map[i.state]++);
    map.forEach((v, i) => {
      a[i].innerHTML = v;
    });
  };

  dataService.addEventListener("active", (e) => {
    activityState.title = "Online";
  });
  dataService.addEventListener("inactive", (e) => {
    activityState.title = "Offline";
    update([]);
  });
  dataService.addEventListener("update", (e) => {
    update(e.detail);
  });

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
  document.body.appendChild(createTable());
};

document.body ? main() : addEventListener("load", main);
