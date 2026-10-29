import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";

test("renders the brand", () => {
  render(<App />);
  expect(screen.getByText("Ticket Flow")).toBeTruthy();
});
