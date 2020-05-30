// The same as before...
use lv2::prelude::*;
use wmidi::*;

#[derive(PortCollection)]
pub struct Ports {
    input: InputPort<AtomPort>,
    output: OutputPort<AtomPort>,
}

#[derive(FeatureCollection)]
pub struct Features<'a> {
    map: LV2Map<'a>,
}

#[derive(URIDCollection)]
pub struct URIDs {
    atom: AtomURIDCollection,
    midi: MidiURIDCollection,
    unit: UnitURIDCollection,
}

pub struct ActiveNote {
  channel: Channel,
  note: Note,
  velocity: Velocity,
}

#[uri("https://github.com/blacksph3re/midilatch")]
pub struct Midilatch {
    urids: URIDs,
    active_notes: Vec<ActiveNote>,
    keys_pressed: u8,
}

impl Plugin for Midilatch {
    type Ports = Ports;

    type InitFeatures = Features<'static>;
    type AudioFeatures = ();

    fn new(_plugin_info: &PluginInfo, features: &mut Features<'static>) -> Option<Self> {
        Some(Self {
            urids: features.map.populate_collection()?,
            active_notes: Vec::with_capacity(100),
            keys_pressed: 0,
        })
    }

    // This plugin works similar to the previous one: It iterates over the events in the input port. However, it only needs to write one or two messages instead of blocks of audio.
    fn run(&mut self, ports: &mut Ports, _: &mut ()) {
        // Get the reading handle of the input sequence.
        let input_sequence = ports
            .input
            .read(self.urids.atom.sequence, self.urids.unit.beat)
            .unwrap();

        // Initialise the output sequence and get the writing handle.
        let mut output_sequence = ports
            .output
            .init(
                self.urids.atom.sequence,
                TimeStampURID::Frames(self.urids.unit.frame),
            )
            .unwrap();

        for (timestamp, atom) in input_sequence {
            // Retrieve the message.
            let message = if let Some(message) = atom.read(self.urids.midi.wmidi, ()) {
                message
            } else {
                output_sequence.forward(timestamp, atom);
                continue;
            };

            match message {
                MidiMessage::NoteOn(channel, note, velocity) => {
                    // If we have a new note but no new keys are pressed, send away all active notes and deactivate them
                    if self.keys_pressed == 0 && !self.active_notes.is_empty() {
                      for note in &self.active_notes {
                        output_sequence
                          .init(timestamp, self.urids.midi.wmidi, MidiMessage::NoteOff(note.channel, note.note, note.velocity))
                          .unwrap();
                      }
                      self.active_notes.clear();
                    }

                    // Add the note to internal storage
                    // Also register the keypress
                    self.active_notes.push( ActiveNote{ channel, note, velocity });
                    self.keys_pressed = match self.keys_pressed.checked_add(1) {
                      None => self.keys_pressed,
                      Some(number) => number,
                    };

                    // Forward the noteon
                    output_sequence.forward(timestamp, atom);
                }
                MidiMessage::NoteOff(_channel, _note, _velocity) => {
                    // Decrease the number of pressed keys
                    self.keys_pressed = match self.keys_pressed.checked_sub(1) {
                      None => 0,
                      Some(number) => number,
                    };
                }
                _ => {
                  output_sequence.forward(timestamp, atom);
                },
            }
        }
    }
}

lv2_descriptors!(Midilatch);
