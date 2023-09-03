export function singleFlight(asyncFunc) {
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
}
