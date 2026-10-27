-- AKL-M040-02: Lifecycle-Service – Kill-Switch und Akteur im Transition-Log.
-- Rein additiv; bestehende Zeilen erhalten den Akteur 'system'.

-- Genau eine Zeile (id = true).
CREATE TABLE device_settings (
    id                     BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),
    kill_switch            BOOLEAN NOT NULL DEFAULT false,
    kill_switch_changed_at TIMESTAMPTZ,
    kill_switch_changed_by TEXT
);
INSERT INTO device_settings DEFAULT VALUES;

ALTER TABLE device_state_transitions
    ADD COLUMN actor TEXT NOT NULL DEFAULT 'system' CHECK (char_length(actor) BETWEEN 1 AND 128);

ALTER TABLE device_profiles
    ADD CONSTRAINT device_profiles_display_name_len CHECK (char_length(display_name) BETWEEN 1 AND 64);
