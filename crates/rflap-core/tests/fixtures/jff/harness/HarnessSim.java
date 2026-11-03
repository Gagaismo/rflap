import automata.Automaton;
import automata.AutomatonSimulator;
import automata.Configuration;
import automata.State;
import automata.Transition;
import automata.fsa.FSAStepWithClosureSimulator;
import automata.mealy.MealyConfiguration;
import automata.mealy.MealyStepByStateSimulator;
import automata.mealy.MooreMachine;
import automata.mealy.MooreStepByStateSimulator;
import automata.pda.PDAStepByStateSimulator;
import automata.turing.TMSimulator;
import automata.turing.TuringMachine;
import file.XMLCodec;
import java.io.File;
import java.io.PrintWriter;
import java.io.Serializable;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;

/**
 * Harness that loads a .jff with JFLAP 7.1's own XMLCodec, prints its graph in a canonical
 * form and runs inputs in JFLAP's own simulators (the ones Input > Run uses: with closure for
 * a finite automaton, step by state for the others; acceptance by final state).
 *
 *   java -Djava.awt.headless=true -cp JFLAP7_1.JAR HarnessSim.java &lt;in.jff&gt; &lt;inputs.txt&gt; &lt;out.txt&gt;
 *
 * inputs.txt: one input per line (UTF-8); a Turing machine with several tapes takes one input
 * per tape separated by a tab; an empty line is the empty string. out.txt (UTF-8) gets:
 *   type, state, transition lines (sorted by id), then one line per input:
 *   run &lt;n&gt; accept|reject|undecided [output=&lt;text&gt;]
 */
public class HarnessSim {
    static final int MAX_CONFIGS = 20000;

    public static void main(String[] args) throws Exception {
        Serializable s = new XMLCodec().decode(new File(args[0]), null);
        List<String> inputs = Files.readAllLines(new File(args[1]).toPath(), StandardCharsets.UTF_8);
        try (PrintWriter out = new PrintWriter(new File(args[2]), "UTF-8")) {
            Automaton a = (Automaton) s;
            out.println("type " + s.getClass().getSimpleName());
            out.println("initial " + (a.getInitialState() == null ? "none" : a.getInitialState().getID()));
            State[] states = a.getStates();
            Arrays.sort(states, Comparator.comparingInt(State::getID));
            for (State st : states) {
                String output = a instanceof MooreMachine ? " output=" + ((MooreMachine) a).getOutput(st) : "";
                out.println("state " + st.getID() + " name=" + st.getName() + " label=" + st.getLabel()
                    + " final=" + a.isFinalState(st) + output);
            }
            List<String> ts = new ArrayList<>();
            for (Transition t : a.getTransitions()) {
                ts.add("transition " + t.getFromState().getID() + " " + t.getToState().getID() + " " + t.getDescription());
            }
            ts.sort(null);
            ts.forEach(out::println);
            for (int i = 0; i < inputs.size(); i++) {
                out.println("run " + i + " " + run(a, inputs.get(i)));
            }
        }
    }

    static String run(Automaton a, String line) throws Exception {
        if (a instanceof TuringMachine) {
            return runTm((TuringMachine) a, line);
        }
        if (a instanceof MooreMachine) {
            return runMealy(new MooreStepByStateSimulator(a), line);
        }
        if (a instanceof automata.mealy.MealyMachine) {
            return runMealy(new MealyStepByStateSimulator(a), line);
        }
        if (a instanceof automata.pda.PushdownAutomaton) {
            return pdaSimulator(a).simulateInput(line) ? "accept" : "reject";
        }
        // what JFLAP's Run uses for a finite automaton (SimulatorFactory): with closure
        return new FSAStepWithClosureSimulator(a).simulateInput(line) ? "accept" : "reject";
    }

    /**
     * JFLAP's PDA simulator asks "Accept by" in a dialog from its constructor, which cannot
     * run headless: allocate it without the constructor and do what the dialog would do.
     */
    static PDAStepByStateSimulator pdaSimulator(Automaton a) throws Exception {
        java.lang.reflect.Field unsafeField = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
        unsafeField.setAccessible(true);
        sun.misc.Unsafe unsafe = (sun.misc.Unsafe) unsafeField.get(null);
        PDAStepByStateSimulator sim = (PDAStepByStateSimulator) unsafe.allocateInstance(PDAStepByStateSimulator.class);
        java.lang.reflect.Field automaton = AutomatonSimulator.class.getDeclaredField("myAutomaton");
        automaton.setAccessible(true);
        automaton.set(sim, a);
        java.lang.reflect.Field configurations = AutomatonSimulator.class.getDeclaredField("myConfigurations");
        configurations.setAccessible(true);
        configurations.set(sim, new java.util.HashSet());
        sim.setAcceptByFinalState();
        return sim;
    }

    static String runMealy(AutomatonSimulator sim, String line) {
        List<Configuration> frontier = new ArrayList<>(Arrays.asList(sim.getInitialConfigurations(line)));
        int seen = 0;
        while (!frontier.isEmpty()) {
            List<Configuration> next = new ArrayList<>();
            for (Configuration c : frontier) {
                if (c.isAccept()) {
                    return "accept output=" + ((MealyConfiguration) c).getOutput();
                }
                next.addAll(sim.stepConfiguration(c));
            }
            frontier = next;
            seen += frontier.size();
            if (seen > MAX_CONFIGS) {
                return "undecided";
            }
        }
        return "reject";
    }

    static String runTm(TuringMachine tm, String line) {
        TMSimulator sim = new TMSimulator(tm);
        Configuration[] start = tm.tapes() == 1
            ? sim.getInitialConfigurations(line)
            : sim.getInitialConfigurations(line.split("\t", -1));
        List<Configuration> frontier = new ArrayList<>(Arrays.asList(start));
        int seen = 0;
        while (!frontier.isEmpty()) {
            List<Configuration> next = new ArrayList<>();
            for (Configuration c : frontier) {
                if (c.isAccept()) {
                    return "accept";
                }
                next.addAll(sim.stepConfiguration(c));
            }
            frontier = next;
            seen += frontier.size();
            if (seen > MAX_CONFIGS) {
                return "undecided";
            }
        }
        return "reject";
    }
}
