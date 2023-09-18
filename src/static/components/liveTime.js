export const createLiveTime = (timestamp) => {
  const el = document.createElement("time");
  el.className = "liveTime";
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
