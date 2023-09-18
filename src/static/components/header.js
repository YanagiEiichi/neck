import { createStateBar } from "./stateBar.js";
import { createActivityState } from "./activityState.js";

export const createHeader = () => {
  const header = document.createElement("header");

  const img = new Image();
  img.src = "./neck.png";
  img.alt = "logo";
  header.appendChild(img);
  header.appendChild(createStateBar());
  header.appendChild(createActivityState());
  return header;
};
