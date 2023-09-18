export const singleFlight = (asyncFunc) => {
  let tasks = [];
  const push = () => {
    let task = asyncFunc();
    task.finally(() => tasks.pop());
    return task;
  };
  return () => {
    switch (tasks.length) {
      case 0:
        tasks[0] = push();
        return tasks[0];
      case 1:
        tasks[1] = tasks[0].then(push, push);
      case 2:
        return tasks[1];
    }
  };
};

export const renderState = (s) => {
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

export const groupBy = (list, name) => {
  const map = Object.create(null);
  list.forEach((i) => {
    let v = i[name];
    if (v in map) {
      map[v]++;
    } else {
      map[v] = 1;
    }
  });
  return map;
};
