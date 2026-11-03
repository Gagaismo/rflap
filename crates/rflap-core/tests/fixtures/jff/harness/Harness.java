import automata.Note;
import automata.State;
import automata.Transition;
import automata.fsa.FSATransition;
import automata.fsa.FiniteStateAutomaton;
import file.XMLCodec;
import java.awt.Point;
import java.io.File;

/**
 * Harness around JFLAP 7.1's own file code (our source, JFLAP's classes).
 *
 *   java -Djava.awt.headless=true -cp JFLAP7_1.JAR Harness.java gen <out.jff>
 *       builds a sample FSA with JFLAP classes and saves it with JFLAP's XMLCodec
 *   java -Djava.awt.headless=true -cp JFLAP7_1.JAR Harness.java load <in.jff>
 *       loads a file with JFLAP's XMLCodec and prints a summary
 */
public class Harness {
    public static void main(String[] args) throws Exception {
        if (args.length == 2 && args[0].equals("gen")) {
            gen(new File(args[1]));
        } else if (args.length == 2 && args[0].equals("load")) {
            load(new File(args[1]));
        } else {
            System.err.println("usage: Harness gen|load <file>");
            System.exit(2);
        }
    }

    static void gen(File out) {
        FiniteStateAutomaton fsa = new FiniteStateAutomaton();
        State q0 = fsa.createState(new Point(50, 100));
        State q1 = fsa.createState(new Point(150, 100));
        State q2 = fsa.createState(new Point(250, 150));
        q1.setName("middle");
        q2.setLabel("accept");
        fsa.setInitialState(q0);
        fsa.addFinalState(q2);
        fsa.addTransition(new FSATransition(q0, q1, "a"));
        fsa.addTransition(new FSATransition(q1, q2, ""));
        fsa.addTransition(new FSATransition(q2, q2, "b"));
        Transition curved = new FSATransition(q1, q0, "ab");
        curved.setControl(new Point(100, 40));
        fsa.addTransition(curved);
        fsa.addNote(new Note(new Point(20, 20), "hello <note> & co"));
        new XMLCodec().encode(fsa, out, null);
    }

    static void load(File in) {
        FiniteStateAutomaton fsa = (FiniteStateAutomaton) new XMLCodec().decode(in, null);
        System.out.println("states=" + fsa.getStates().length
            + " transitions=" + fsa.getTransitions().length
            + " finals=" + fsa.getFinalStates().length
            + " initial=" + (fsa.getInitialState() == null ? "none" : fsa.getInitialState().getID())
            + " notes=" + fsa.getNotes().size());
        for (State s : fsa.getStates()) {
            System.out.println("state " + s.getID() + " name=" + s.getName() + " label=" + s.getLabel()
                + " at=(" + s.getPoint().getX() + "," + s.getPoint().getY() + ")");
        }
        for (Transition t : fsa.getTransitions()) {
            System.out.println("transition " + t.getFromState().getID() + "->" + t.getToState().getID()
                + " label=[" + ((FSATransition) t).getLabel() + "] control=" + t.getControl());
        }
    }
}
