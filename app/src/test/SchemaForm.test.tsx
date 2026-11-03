import { fireEvent, render, screen } from "@testing-library/react";
import i18n from "@/i18n";
import { SchemaForm } from "@/settings/SchemaForm";
import type { JSONSchema } from "@/lib/schema";
import type { JsonValue } from "@/lib/backend";
import settingsSchema from "../../../config/defaults/schemas/settings.schema.json";
import themeSchema from "../../../config/defaults/schemas/theme.schema.json";
import { createMockBackend } from "@/lib/backend.mock";

const ctx = { refIds: (f: string) => (f === "themes" ? ["dark", "light"] : ["cash-6max", "mtt"]) };

describe("SchemaForm", () => {
  beforeAll(() => i18n.changeLanguage("fr"));

  it("renders the settings schema with translated labels and the right widgets", async () => {
    const value = (await createMockBackend().configGet("settings", "settings")) as JsonValue;
    const onChange = vi.fn();
    render(<SchemaForm schema={settingsSchema as JSONSchema} value={value} onChange={onChange} ctx={ctx} />);
    expect(screen.getByText("Apparence")).toBeTruthy();
    const density = screen.getByLabelText("Densité") as HTMLSelectElement;
    expect(density.value).toBe("comfortable");
    fireEvent.change(density, { target: { value: "compact" } });
    expect(onChange.mock.calls[0][0].appearance.density).toBe("compact");
    // x-ref-family select lists the theme files.
    const theme = screen.getByLabelText("Thème") as HTMLSelectElement;
    expect([...theme.options].map((o) => o.value)).toEqual(["dark", "light"]);
    // Integer field.
    expect((screen.getByLabelText("Threads de calcul") as HTMLInputElement).type).toBe("number");
    // module_settings is hidden (x-widget: hidden).
    expect(screen.queryByText("Réglages des modules")).toBeNull();
  });

  it("uses colour widgets for $ref'd colour tokens and shows issues", async () => {
    const value = (await createMockBackend().configGet("themes", "dark")) as JsonValue;
    render(
      <SchemaForm
        schema={themeSchema as JSONSchema}
        value={value}
        onChange={() => undefined}
        ctx={ctx}
        issues={[{ family: "themes", id: "dark", pointer: "/colors/primary", message: "bad colour" }]}
      />,
    );
    expect(screen.getByLabelText("Primaire")).toBeTruthy();
    expect(screen.getAllByRole("alert")[0].textContent).toBe("bad colour");
    expect(document.querySelectorAll('input[type="color"]')).toHaveLength(12);
  });
});
