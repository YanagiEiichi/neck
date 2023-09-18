export const createTableTip = () => {
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
