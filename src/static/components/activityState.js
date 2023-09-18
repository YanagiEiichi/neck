import { dataService } from "../dataService.js";

export const createActivityState = () => {
  const div = document.createElement("div");
  div.className = "activityState";

  dataService.addEventListener("active", () => {
    div.title = "Online";
    div.classList.add("living");
  });
  dataService.addEventListener("inactive", () => {
    div.title = "Offline";
    div.classList.remove("living");
  });

  return div;
};
