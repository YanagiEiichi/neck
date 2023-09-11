import { dataService } from "./dataService.js";

const renderState = (s) => {
  switch (s) {
    case 0:
      return `<b style="color: gray;">Waiting</b>`;
    case 1:
      return `<b style="color: gray;">Connecting</b>`;
    case 2:
      return `<b style="color: green;">Established</b>`;
    default:
      return `<b style="color: red;">Unknown</b>`;
  }
};

const renderTime = (timestamp) => {
  const el = document.createElement("time");
  let nodes = [];
  let update = () => {
    let list = Math.floor((Date.now() - timestamp) / 1000)
      .toString()
      .split(/(?=(?:...)*$)/);
    while (list.length > nodes.length) {
      let n = new Text();
      if (nodes.length > 0) {
        el.appendChild(document.createElement("s"));
      }
      el.appendChild(n);
      nodes.push(n);
    }
    const s = getSelection();
    let inRange = el.contains(s.anchorNode) && el.contains(s.focusNode);
    for (let i = 0; i < list.length; i++) {
      if (nodes[i].data !== list[i]) nodes[i].data = list[i];
    }
    if (inRange) s.selectAllChildren(el);
  };
  update();
  let timer = setInterval(() => {
    if (!el.parentNode) return clearInterval(timer);
    update();
  }, 100);
  return el;
};

const renderTable = async () => {
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
        cells[5].appendChild(renderTime(data.timestamp));
      }
    };
    row.update(data);
    return row;
  };

  dataService.addEventListener("update", (e) => {
    let list = e.detail;
    let m = new Map(list.map((i) => [String(i.id), i]));
    let { rows } = table;
    for (let i = 1; i < rows.length; i++) {
      let row = rows[i];
      let data = m.get(row.dataset.id);
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
  });

  let row = table.insertRow();
  row.insertCell().textContent = "ID";
  row.insertCell().textContent = "Type";
  row.insertCell().textContent = "State";
  row.insertCell().textContent = "Host";
  row.insertCell().textContent = "From";
  row.insertCell().textContent = "Uptime";
  document.body.append(table);
};

const renderHeader = () => {
  let header = document.createElement("header");
  let length = 3;

  const a = Array.from({ length }, (_, index) => {
    if (index) header.appendChild(new Text(", "));
    header.insertAdjacentHTML("beforeEnd", renderState(index));
    header.appendChild(new Text(": "));
    let v = document.createElement("var");
    v.textContent = 0;
    header.appendChild(v);
    return v;
  });

  let activityState = document.createElement("div");
  activityState.className = "activityState";
  header.appendChild(activityState);
  dataService.addEventListener("active", (e) => {
    activityState.classList.add("active");
    activityState.classList.remove("inactive");
  });

  dataService.addEventListener("inactive", (e) => {
    activityState.classList.remove("active");
    activityState.classList.add("inactive");
  });

  dataService.addEventListener("update", (e) => {
    const { detail } = e;
    let map = Array(length).fill(0);
    detail.forEach((i) => map[i.state]++);
    map.forEach((v, i) => {
      a[i].innerHTML = v;
    });
  });

  document.body.append(header);
};

const main = async () => {
  renderHeader();
  renderTable();
};

document.body ? main() : addEventListener("load", main);
