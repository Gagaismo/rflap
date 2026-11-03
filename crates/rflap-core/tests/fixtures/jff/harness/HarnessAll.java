import automata.Automaton;
import automata.State;
import automata.Transition;
import automata.mealy.MealyMachine;
import automata.mealy.MealyTransition;
import automata.mealy.MooreMachine;
import automata.mealy.MooreTransition;
import automata.pda.PDATransition;
import automata.pda.PushdownAutomaton;
import automata.turing.TMState;
import automata.turing.TMTransition;
import automata.turing.TuringMachine;
import file.XMLCodec;
import grammar.Grammar;
import grammar.Production;
import grammar.UnboundGrammar;
import java.awt.Point;
import java.io.File;
import java.io.Serializable;
import regular.RegularExpression;

/**
 * Harness around JFLAP 7.1's own file code, for every structure type of part B of T15.
 *
 *   java -Djava.awt.headless=true -cp JFLAP7_1.JAR HarnessAll.java gen <outdir>
 *       builds one structure per type with JFLAP classes and saves it with JFLAP's XMLCodec
 *       (pda.jff, turing1.jff, turing2.jff, turing_block.jff, turing_nested.jff, mealy.jff,
 *       moore.jff, grammar.jff, re.jff)
 *   java -Djava.awt.headless=true -cp JFLAP7_1.JAR HarnessAll.java load <in.jff>
 *       loads a file with JFLAP's XMLCodec and prints a summary of what JFLAP read
 */
public class HarnessAll {
    public static void main(String[] args) throws Exception {
        if (args.length == 2 && args[0].equals("gen")) {
            gen(new File(args[1]));
        } else if (args.length == 2 && args[0].equals("load")) {
            load(new File(args[1]));
        } else {
            System.err.println("usage: HarnessAll gen <dir> | load <file>");
            System.exit(2);
        }
    }

    static void save(Serializable s, File dir, String name) {
        new XMLCodec().encode(s, new File(dir, name), null);
    }

    static void gen(File dir) {
        dir.mkdirs();

        // a pushdown automaton with a curved transition, a multi-symbol push and a note
        PushdownAutomaton pda = new PushdownAutomaton();
        State p0 = pda.createState(new Point(50, 100));
        State p1 = pda.createState(new Point(150, 100));
        State p2 = pda.createState(new Point(250, 100));
        p1.setName("pushing");
        p2.setLabel("done");
        pda.setInitialState(p0);
        pda.addFinalState(p2);
        pda.addTransition(new PDATransition(p0, p0, "a", "", "A"));
        pda.addTransition(new PDATransition(p0, p1, "", "", ""));
        pda.addTransition(new PDATransition(p1, p1, "b", "A", ""));
        Transition multi = new PDATransition(p1, p2, "", "Z", "XY");
        multi.setControl(new Point(200, 40));
        pda.addTransition(multi);
        pda.addNote(new automata.Note(new Point(20, 20), "pda & <note>"));
        save(pda, dir, "pda.jff");

        // one tape
        TuringMachine t1 = new TuringMachine(1);
        TMState a = t1.createTMState(new Point(50, 100));
        TMState b = t1.createTMState(new Point(150, 100));
        TMState c = t1.createTMState(new Point(250, 100));
        b.setName("flip");
        c.setLabel("halt");
        t1.setInitialState(a);
        t1.addFinalState(c);
        t1.addTransition(new TMTransition(a, b, "0", "1", "R"));
        t1.addTransition(new TMTransition(b, b, "1", "0", "R"));
        t1.addTransition(new TMTransition(b, c, "", "", "S"));
        t1.addTransition(new TMTransition(a, a, "~", "~", "L"));
        t1.addTransition(new TMTransition(c, a, "!x", "y", "R"));
        save(t1, dir, "turing1.jff");

        // two tapes
        TuringMachine t2 = new TuringMachine(2);
        TMState d = t2.createTMState(new Point(50, 100));
        TMState e = t2.createTMState(new Point(200, 100));
        t2.setInitialState(d);
        t2.addFinalState(e);
        t2.addTransition(new TMTransition(d, d, new String[] {"a", ""}, new String[] {"a", "a"}, new String[] {"R", "R"}));
        t2.addTransition(new TMTransition(d, d, new String[] {"b", ""}, new String[] {"b", "b"}, new String[] {"R", "R"}));
        t2.addTransition(new TMTransition(d, e, new String[] {"", ""}, new String[] {"", ""}, new String[] {"S", "L"}));
        save(t2, dir, "turing2.jff");

        // a building block (explicit internal name: the default MachineN collides when JFLAP reloads)
        // whose inner machine writes a 1 and moves right
        TuringMachine inner = new TuringMachine(1);
        TMState i0 = inner.createTMState(new Point(40, 60));
        TMState i1 = inner.createTMState(new Point(140, 60));
        inner.setInitialState(i0);
        inner.addFinalState(i1);
        inner.addTransition(new TMTransition(i0, i1, "", "1", "R"));
        TuringMachine outer = new TuringMachine(1);
        TMState o0 = outer.createTMState(new Point(50, 100));
        TMState o1 = outer.createTMState(new Point(200, 100));
        TMState o2 = outer.createTMState(new Point(350, 100));
        o1.setInnerTM(inner);
        o1.setInternalName("Increment");
        o1.setName("writeone");
        outer.setInitialState(o0);
        outer.addFinalState(o2);
        outer.addTransition(new TMTransition(o0, o1, "a", "a", "S"));
        TMTransition bt = new TMTransition(o1, o2, "", "~", "S");
        bt.setBlockTransition(true);
        outer.addTransition(bt);
        save(outer, dir, "turing_block.jff");

        // a block inside a block
        TuringMachine leaf = new TuringMachine(1);
        TMState l0 = leaf.createTMState(new Point(40, 60));
        TMState l1 = leaf.createTMState(new Point(140, 60));
        leaf.setInitialState(l0);
        leaf.addFinalState(l1);
        leaf.addTransition(new TMTransition(l0, l1, "", "x", "L"));
        TuringMachine mid = new TuringMachine(1);
        TMState m0 = mid.createTMState(new Point(40, 60));
        TMState m1 = mid.createTMState(new Point(140, 60));
        m1.setInnerTM(leaf);
        m1.setInternalName("Leaf");
        mid.setInitialState(m0);
        mid.addFinalState(m1);
        mid.addTransition(new TMTransition(m0, m1, "", "", "S"));
        TuringMachine top = new TuringMachine(1);
        TMState u0 = top.createTMState(new Point(50, 100));
        TMState u1 = top.createTMState(new Point(200, 100));
        u1.setInnerTM(mid);
        u1.setInternalName("Mid");
        top.setInitialState(u0);
        top.addFinalState(u1);
        top.addTransition(new TMTransition(u0, u1, "a", "a", "R"));
        save(top, dir, "turing_nested.jff");

        MealyMachine mealy = new MealyMachine();
        State e0 = mealy.createState(new Point(50, 100));
        State e1 = mealy.createState(new Point(150, 100));
        e0.setName("start");
        mealy.setInitialState(e0);
        mealy.addTransition(new MealyTransition(e0, e1, "a", "01"));
        mealy.addTransition(new MealyTransition(e1, e0, "b", ""));
        mealy.addTransition(new MealyTransition(e1, e1, "", "z"));
        save(mealy, dir, "mealy.jff");

        MooreMachine moore = new MooreMachine();
        State r0 = moore.createState(new Point(50, 100));
        State r1 = moore.createState(new Point(150, 100));
        State r2 = moore.createState(new Point(250, 100));
        moore.setInitialState(r0);
        // a MooreTransition writes its output into the destination state, so set outputs last
        moore.addTransition(new MooreTransition(r0, r1, "a", ""));
        moore.addTransition(new MooreTransition(r1, r2, "b", ""));
        moore.addTransition(new MooreTransition(r2, r0, "a", ""));
        moore.setOutput(r0, "x");
        moore.setOutput(r1, "yy");
        save(moore, dir, "moore.jff");

        UnboundGrammar g = new UnboundGrammar();
        g.addProduction(new Production("S", "aSb"));
        g.addProduction(new Production("S", ""));
        g.addProduction(new Production("aS", "Ab"));
        save(g, dir, "grammar.jff");

        save(new RegularExpression("(a+b)*abb"), dir, "re.jff");
    }

    static String esc(String s) {
        return s.replace("\u25a1", "[]");
    }

    static void load(File in) {
        Serializable s = new XMLCodec().decode(in, null);
        if (s instanceof TuringMachine) {
            printTm("tm", (TuringMachine) s, "");
        } else if (s instanceof Automaton) {
            Automaton a = (Automaton) s;
            System.out.println(s.getClass().getSimpleName() + " states=" + a.getStates().length
                + " transitions=" + a.getTransitions().length
                + " finals=" + a.getFinalStates().length
                + " initial=" + (a.getInitialState() == null ? "none" : a.getInitialState().getID())
                + " notes=" + a.getNotes().size());
            for (State st : a.getStates()) {
                String out = a instanceof MooreMachine ? " output=[" + ((MooreMachine) a).getOutput(st) + "]" : "";
                System.out.println("  state " + st.getID() + " name=" + st.getName() + " label=" + st.getLabel()
                    + " at=(" + st.getPoint().getX() + "," + st.getPoint().getY() + ")" + out);
            }
            for (Transition t : a.getTransitions()) {
                System.out.println("  transition " + t.getFromState().getID() + "->" + t.getToState().getID()
                    + " " + t.getDescription() + " control=" + t.getControl());
            }
        } else if (s instanceof Grammar) {
            Grammar g = (Grammar) s;
            System.out.println("Grammar productions=" + g.getProductions().length + " start=" + g.getStartVariable());
            for (Production p : g.getProductions()) {
                System.out.println("  production [" + p.getLHS() + "] -> [" + p.getRHS() + "]");
            }
        } else if (s instanceof RegularExpression) {
            System.out.println("RegularExpression [" + ((RegularExpression) s).asString() + "]");
        } else {
            System.out.println("other " + s.getClass().getName());
        }
    }

    static void printTm(String title, TuringMachine tm, String indent) {
        System.out.println(indent + title + " tapes=" + tm.tapes() + " states=" + tm.getStates().length
            + " transitions=" + tm.getTransitions().length + " finals=" + tm.getFinalStates().length
            + " initial=" + (tm.getInitialState() == null ? "none" : tm.getInitialState().getID())
            + " notes=" + tm.getNotes().size());
        for (State st : tm.getStates()) {
            TMState ts = (TMState) st;
            System.out.println(indent + "  state " + ts.getID() + " name=" + ts.getName() + " internal=" + ts.getInternalName()
                + " at=(" + ts.getPoint().getX() + "," + ts.getPoint().getY() + ")"
                + " innerStates=" + ts.getInnerTM().getStates().length);
            if (ts.getInnerTM().getStates().length > 0) {
                printTm("inner of " + ts.getID(), ts.getInnerTM(), indent + "    ");
            }
        }
        for (Transition t : tm.getTransitions()) {
            TMTransition tt = (TMTransition) t;
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < tt.tapes(); i++) {
                sb.append(" [").append(esc(tt.getRead(i))).append(";").append(esc(tt.getWrite(i)))
                    .append(",").append(tt.getDirection(i)).append("]");
            }
            System.out.println(indent + "  transition " + t.getFromState().getID() + "->" + t.getToState().getID()
                + sb + " block=" + tt.isBlockTransition());
        }
    }
}
