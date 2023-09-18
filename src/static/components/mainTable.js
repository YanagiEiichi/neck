import { createLiveTime } from "./liveTime.js";
import { renderState } from "../utils.js";
import { dataService } from "../dataService.js";

export const createMainTable = () => {
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
