import automata.*;
import automata.fsa.*;
import automata.pda.*;
import automata.turing.*;
import grammar.*;
import grammar.cfg.*;
import grammar.reg.*;
import gui.grammar.GrammarTableModel;
import java.awt.Point;
import java.util.*;

/** Differential harness for T15a: prints what JFLAP 7.1 computes. Run:
 *  java -Djava.awt.headless=true -cp JFLAP7_1.JAR Harness.java > expected.txt */
public class Harness {
    static String show(Production[] ps) {
        StringBuilder sb = new StringBuilder();
        for (Production p : ps) sb.append(p.getLHS()).append("->").append(p.getRHS()).append(" ; ");
        return sb.toString().trim();
    }
    static String show(Collection<Production> ps) {
        return show(ps.toArray(new Production[0]));
    }

    static State[] states(Automaton a, int n) {
        State[] s = new State[n];
        for (int i = 0; i < n; i++) s[i] = a.createState(new Point(100 * i, 0));
        return s;
    }

    static String pdaTransitions(Automaton a) {
        StringBuilder sb = new StringBuilder();
        for (Transition t : a.getTransitions()) {
            PDATransition p = (PDATransition) t;
            sb.append(t.getFromState().getID()).append('>').append(t.getToState().getID()).append(' ')
              .append('[').append(p.getInputToRead()).append(',').append(p.getStringToPop()).append(';')
              .append(p.getStringToPush()).append("] ; ");
        }
        return sb.toString().trim();
    }

    static FiniteStateAutomaton fa(int n, int initial, int[] finals, Object[][] edges) {
        FiniteStateAutomaton f = new FiniteStateAutomaton();
        State[] s = states(f, n);
        f.setInitialState(s[initial]);
        for (int x : finals) f.addFinalState(s[x]);
        for (Object[] e : edges) f.addTransition(new FSATransition(s[(Integer) e[0]], s[(Integer) e[1]], (String) e[2]));
        return f;
    }

    /** JFLAP keeps transitions in a HashSet keyed by identity hashes, so its order is arbitrary; the
     *  harness feeds them in insertion order, which is what Rflap iterates. */
    static List<Transition> order = new ArrayList<>();

    static PushdownAutomaton pda(int n, int[] finals, Object[][] edges) {
        order = new ArrayList<>();
        PushdownAutomaton p = new PushdownAutomaton();
        State[] s = states(p, n);
        p.setInitialState(s[0]);
        for (int x : finals) p.addFinalState(s[x]);
        for (Object[] e : edges) {
            Transition t = new PDATransition(s[(Integer) e[0]], s[(Integer) e[1]], (String) e[2], (String) e[3], (String) e[4]);
            order.add(t);
            p.addTransition(t);
        }
        return p;
    }

    /** What the JFLAP GUI does for PDA -> grammar: reveal every transition, purge, simplify. */
    static void pdaToCfg(String tag, PushdownAutomaton p) {
        PDAToCFGConverter c = new PDAToCFGConverter();
        c.initializeConverter();
        GrammarTableModel model = new GrammarTableModel();
        for (Transition t : order)
            for (Object o : c.createProductionsForTransition(t, p)) model.addProduction((Production) o);
        int before = model.getProductions().length;
        c.purgeProductions(p, model);
        List<Production> out = new ArrayList<>();
        for (Production pr : model.getProductions()) out.add(c.getSimplifiedProduction(pr));
        System.out.println(tag + " raw " + before + " kept " + out.size());
        System.out.println(tag + " out " + show(out));
    }

    static void tm(String tag, TuringMachine m) {
        TuringToGrammarConverter c = new TuringToGrammarConverter();
        State init = m.getInitialState();
        System.out.println(tag + " init " + show(c.createProductionsForInit(init, m.getTransitions())));
        for (Transition t : m.getTransitions())
            System.out.println(tag + " trans " + t.getFromState().getID() + ">" + t.getToState().getID() + " "
                + show(c.createProductionsForTransition(t, m.getFinalStates())));
    }

    static TuringMachine tm(int n, int[] finals, Object[][] edges) {
        TuringMachine m = new TuringMachine(1);
        State[] s = states(m, n);
        m.setInitialState(s[0]);
        for (int x : finals) m.addFinalState(s[x]);
        for (Object[] e : edges)
            m.addTransition(new TMTransition(s[(Integer) e[0]], s[(Integer) e[1]], (String) e[2], (String) e[3], (String) e[4]));
        return m;
    }

    public static void main(String[] args) throws Exception {
        // FA -> regular grammar
        FSAToRegularGrammarConverter fc = new FSAToRegularGrammarConverter();
        FiniteStateAutomaton dfa = fa(2, 0, new int[] {1}, new Object[][] {{0, 1, "a"}, {0, 0, "b"}, {1, 1, "a"}, {1, 0, "b"}});
        System.out.println("FA2G dfa " + show(fc.convertToRegularGrammar(dfa).getProductions()));
        FiniteStateAutomaton f3 = fa(3, 2, new int[] {0}, new Object[][] {{2, 0, "a"}, {0, 1, "b"}, {1, 2, "c"}});
        System.out.println("FA2G ids " + show(fc.convertToRegularGrammar(f3).getProductions()));
        FiniteStateAutomaton f4 = fa(3, 0, new int[] {2}, new Object[][] {{0, 0, "a"}, {0, 1, ""}, {1, 2, "ab"}});
        System.out.println("FA2G lambda " + show(fc.convertToRegularGrammar(f4).getProductions()));

        // right-linear grammar -> FA
        RightLinearGrammar g = new RightLinearGrammar();
        g.setStartVariable("S");
        for (String[] p : new String[][] {{"S", "aS"}, {"S", "bA"}, {"A", "b"}, {"A", ""}})
            g.addProduction(new Production(p[0], p[1]));
        RightLinearGrammarToFSAConverter rc = new RightLinearGrammarToFSAConverter();
        FiniteStateAutomaton rfa = new FiniteStateAutomaton();
        rc.createStatesForConversion(g, rfa);
        StringBuilder sb = new StringBuilder();
        for (State s : rfa.getStates())
            sb.append(s.getID()).append('=').append(s.getLabel()).append(rfa.isFinalState(s) ? "(final)" : "")
              .append(s == rfa.getInitialState() ? "(initial)" : "").append(' ');
        System.out.println("RL2FA states " + sb.toString().trim());
        sb = new StringBuilder();
        for (Production p : g.getProductions()) {
            FSATransition t = (FSATransition) rc.getTransitionForProduction(p);
            sb.append(t.getFromState().getID()).append('>').append(t.getToState().getID()).append('[').append(t.getLabel()).append("] ; ");
        }
        System.out.println("RL2FA transitions " + sb.toString().trim());

        // CFG -> PDA
        ContextFreeGrammar cfg = new ContextFreeGrammar();
        cfg.setStartVariable("S");
        cfg.addProduction(new Production("S", "aSb"));
        cfg.addProduction(new Production("S", ""));
        CFGToPDALLConverter ll = new CFGToPDALLConverter();
        PushdownAutomaton llp = new PushdownAutomaton();
        ll.createStatesForConversion(cfg, llp);
        for (Production p : cfg.getProductions()) llp.addTransition(ll.getTransitionForProduction(p));
        System.out.println("LL " + pdaTransitions(llp));
        CFGToPDALRConverter lr = new CFGToPDALRConverter();
        PushdownAutomaton lrp = new PushdownAutomaton();
        lr.createStatesForConversion(cfg, lrp);
        for (Production p : cfg.getProductions()) lrp.addTransition(lr.getTransitionForProduction(p));
        System.out.println("LR " + pdaTransitions(lrp));

        // PDA -> CFG
        pdaToCfg("PDA1", pda(3, new int[] {2}, new Object[][] {
            {0, 0, "a", "Z", "AZ"}, {0, 0, "a", "A", "AA"}, {0, 1, "b", "A", ""}, {1, 1, "b", "A", ""}, {1, 2, "", "Z", ""}}));
        pdaToCfg("PDA2", pda(2, new int[] {1}, new Object[][] {
            {0, 0, "a", "Z", "AZ"}, {0, 0, "a", "A", "AA"}, {0, 0, "b", "A", ""}, {0, 1, "", "Z", ""}}));
        pdaToCfg("PDA3", pda(4, new int[] {3}, new Object[][] {
            {0, 1, "a", "Z", "AZ"}, {1, 1, "a", "A", "AA"}, {1, 2, "b", "A", ""}, {2, 2, "b", "A", ""}, {2, 3, "", "Z", ""}, {0, 3, "c", "Z", ""}}));

        // Turing machine -> grammar
        String sq = "\u25a1";
        tm("TM1", tm(3, new int[] {2}, new Object[][] {{0, 1, "a", "b", "R"}, {1, 2, "a", "a", "L"}}));
        tm("TM2", tm(2, new int[] {1}, new Object[][] {{0, 0, "a", "a", "R"}, {0, 1, "b", "b", "R"}}));
        tm("TM3", tm(3, new int[] {2}, new Object[][] {{0, 1, "a", sq, "R"}, {1, 2, sq, "a", "L"}}));
    }
}
