import grammar.*;
import grammar.cfg.ContextFreeGrammar;
import grammar.parse.*;
import automata.fsa.*;
import automata.*;
import java.awt.Point;
import java.util.*;

/** Differential harness for T5: prints what JFLAP 7.1 computes. Run:
 *  java -Djava.awt.headless=true -cp JFLAP7_1.JAR Harness.java > expected.txt */
public class Harness {
    static ContextFreeGrammar cfg(String... lines) {
        ContextFreeGrammar g = new ContextFreeGrammar();
        for (String l : lines) {
            String[] p = l.split("->", -1);
            if (g.getStartVariable() == null) g.setStartVariable(p[0]);
            g.addProduction(new Production(p[0], p[1]));
        }
        return g;
    }
    static String show(Production[] ps) {
        StringBuilder sb = new StringBuilder();
        for (Production p : ps) sb.append(p.getLHS()).append("->").append(p.getRHS()).append(" ; ");
        return sb.toString().trim();
    }
    static String sorted(Collection<String> c) {
        List<String> l = new ArrayList<>(c);
        Collections.sort(l);
        StringBuilder sb = new StringBuilder("{");
        for (String s : l) sb.append(s.isEmpty() ? "!" : s).append(' ');
        return sb.toString().trim() + "}";
    }

    public static void main(String[] args) throws Exception {
        // lambda
        ContextFreeGrammar l = cfg("S->ABC", "A->aA", "A->", "B->bB", "B->", "C->c", "C->");
        LambdaProductionRemover lr = new LambdaProductionRemover();
        Set set = lr.getCompleteLambdaSet(l);
        System.out.println("LAMBDA set " + sorted(set));
        System.out.println("LAMBDA out " + show(lr.getLambdaProductionlessGrammar(l, set).getProductions()));
        // unit
        ContextFreeGrammar u = cfg("S->A", "S->bb", "A->B", "A->a", "B->C", "B->c", "C->A");
        UnitProductionRemover ur = new UnitProductionRemover();
        System.out.println("UNIT out " + show(ur.getUnitProductionlessGrammar(u, ur.getVariableDependencyGraph(u)).getProductions()));
        ContextFreeGrammar u2 = cfg("S->PQ", "S->a", "P->Q", "Q->A", "A->b", "A->P", "Z->Q", "Z->z", "M->Z", "P->p");
        System.out.println("UNIT2 out " + show(ur.getUnitProductionlessGrammar(u2, ur.getVariableDependencyGraph(u2)).getProductions()));
        ContextFreeGrammar l2 = cfg("S->PQZ", "P->", "Q->QP", "Q->q", "Z->ZM", "Z->", "M->m", "M->");
        Set set2 = lr.getCompleteLambdaSet(l2);
        System.out.println("LAMBDA2 set " + sorted(set2));
        System.out.println("LAMBDA2 out " + show(lr.getLambdaProductionlessGrammar(l2, set2).getProductions()));
        // useless
        ContextFreeGrammar x = cfg("S->AB", "S->a", "A->aA", "B->b", "C->c", "D->E", "E->e");
        System.out.println("USELESS out " + show(UselessProductionRemover.getUselessProductionlessGrammar(x).getProductions()));
        // CNF
        for (String[] lines : new String[][] {
                {"S->aSb", "S->ab"},
                {"S->aSb", "S->ab", "S->ASBc", "A->a", "B->bb"} }) {
            ContextFreeGrammar c = cfg(lines);
            CNFConverter cv = new CNFConverter(c);
            List<Production> table = new ArrayList<>(Arrays.asList(c.getProductions()));
            while (true) {
                List<Integer> need = new ArrayList<>();
                for (int i = 0; i < table.size(); i++) if (!cv.isChomsky(table.get(i))) need.add(i);
                if (need.isEmpty()) break;
                for (int n = need.size() - 1; n >= 0; n--) {
                    int idx = need.get(n);
                    Production[] rep;
                    try { rep = cv.replacements(table.get(idx)); } catch (IllegalArgumentException e) { continue; }
                    table.remove(idx);
                    for (int i = rep.length - 1; i >= 0; i--) table.add(idx, rep[i]);
                }
            }
            System.out.println("CNF pre " + show(table.toArray(new Production[0])));
            System.out.println("CNF out " + show(CNFConverter.convert(table.toArray(new Production[0]))));
        }
        // first/follow, expression grammar
        ContextFreeGrammar e = cfg("E->TX", "X->+TX", "X->", "T->FY", "Y->*FY", "Y->", "F->(E)", "F->a");
        Map f = Operations.first(e);
        Map fo = Operations.follow(e);
        for (String v : new String[] {"E", "X", "T", "Y", "F"}) {
            System.out.println("FIRST " + v + " " + sorted((Set) f.get(v)) + " FOLLOW " + sorted((Set) fo.get(v)));
        }
        LLParseTable t = LLParseTableGenerator.generate(e);
        for (String v : new String[] {"E", "X", "T", "Y", "F"})
            for (String c : new String[] {"(", ")", "*", "+", "a", "$"}) {
                SortedSet s = t.get(v, c);
                if (!s.isEmpty()) System.out.println("LL " + v + " " + c + " " + sorted(s));
            }
        System.out.println("LL1 " + Operations.isLL1(e));
        ContextFreeGrammar lrec = cfg("E->E+T", "E->T", "T->a");
        System.out.println("LL1 left-recursive " + Operations.isLL1(lrec));
        // SLR(1)
        ContextFreeGrammar s = cfg("E->E+T", "E->T", "T->T*F", "T->F", "F->(E)", "F->a");
        Grammar aug = Operations.getAugmentedGrammar(s);
        Map follow = Operations.follow(s);
        FiniteStateAutomaton dfa = new FiniteStateAutomaton();
        Map stateToItems = new HashMap(), itemsToState = new HashMap();
        Production p0 = aug.getProductions()[0];
        Set init = new HashSet();
        init.add(new Production(p0.getLHS(), "\u00b7" + p0.getRHS()));
        init = Operations.closure(aug, init);
        State st0 = dfa.createState(new Point(0, 0));
        dfa.setInitialState(st0);
        stateToItems.put(st0, init); itemsToState.put(init, st0);
        HashSet<State> done = new HashSet<>();
        State[] states = dfa.getStates();
        while (states.length != done.size()) {
            for (State q : states) {
                if (done.contains(q)) continue;
                Set items = (Set) stateToItems.get(q);
                boolean fin = false;
                for (Object o : items) if (((Production) o).getRHS().endsWith("\u00b7")) fin = true;
                if (fin) dfa.addFinalState(q);
                for (String sym : new TreeSet<String>(Arrays.asList(Operations.getCanGoto(items)))) {
                    Set g = Operations.goTo(aug, items, sym);
                    State to = (State) itemsToState.get(g);
                    if (to == null) { to = dfa.createState(new Point(0, 0)); stateToItems.put(to, g); itemsToState.put(g, to); }
                    dfa.addTransition(new FSATransition(q, to, sym));
                }
                done.add(q);
            }
            states = dfa.getStates();
        }
        LRParseTable table = LRParseTableGenerator.generate(aug, dfa, stateToItems, itemsToState, follow);
        System.out.println("SLR states " + dfa.getStates().length);
        for (int r = 0; r < table.getRowCount(); r++) {
            StringBuilder sb = new StringBuilder("SLR " + r + ":");
            for (int c = 1; c < table.getColumnCount(); c++) {
                String v = (String) table.getValueAt(r, c);
                if (!v.isEmpty()) sb.append(' ').append(table.getColumnName(c)).append('=').append(v.replace(' ', '/'));
            }
            System.out.println(sb);
        }
        // brute force
        ContextFreeGrammar ab = cfg("S->aSb", "S->");
        for (String in : new String[] {"aabb", "aab", ""}) {
            BruteParser bp = BruteParser.get(ab, in);
            bp.start();
            while (!bp.isFinished()) Thread.sleep(5);
            ParseNode ans = bp.getAnswer();
            StringBuilder sb = new StringBuilder("BRUTE '" + in + "' -> " + (ans != null));
            for (javax.swing.tree.TreeNode n = ans; n != null; n = n.getParent()) {
                ParseNode pn = (ParseNode) n;
                sb.append(" | ").append(pn.getDerivation()).append(" <- ").append(show(pn.getProductions()));
            }
            System.out.println(sb);
        }
        // CYK
        ContextFreeGrammar cyk = cfg("S->AB", "S->BC", "A->BA", "A->a", "B->CC", "B->b", "C->AB", "C->a");
        CYKParser cp = new CYKParser(cyk);
        for (String in : new String[] {"baaba", "aaa", "b"}) System.out.println("CYK " + in + " " + cp.solve(in));
        System.exit(0);
    }
}
