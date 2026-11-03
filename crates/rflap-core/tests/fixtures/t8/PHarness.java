import file.XMLCodec;
import gui.pumping.CFPumpingLemmaChooser;
import gui.pumping.PumpingLemmaChooser;
import gui.pumping.RegPumpingLemmaChooser;
import java.io.File;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.util.*;
import pumping.*;

/**
 * Pumping lemma harness around JFLAP 7.1's own classes. Run:
 *   java -Dline.separator=$'\n' -Dfile.encoding=UTF-8 -Djava.awt.headless=true -cp JFLAP7_1.JAR PHarness.java <out-dir> > pumping_expected.txt
 * (bash syntax: the files must have LF line ends, JFLAP on Windows writes CR LF otherwise)
 * or, to see how JFLAP reads a file:  ... PHarness.java load <file.jff>
 * It prints, for every predefined language, what JFLAP computes (titles, ranges, cases,
 * the choices of w / decomposition / i for fixed seeds of its random generator, membership
 * of every short string) and saves sample games with JFLAP's own XMLCodec.
 */
public class PHarness {
    static PrintStream out;

    static void setRandom(long seed) throws Exception {
        Field f = LemmaMath.class.getDeclaredField("RAND_GENERATOR");
        f.setAccessible(true);
        f.set(null, new Random(seed));
    }

    static String show(int[] a) {
        if (a == null) return "null";
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < a.length; i++) {
            if (i > 0) sb.append(',');
            sb.append(a[i]);
        }
        return sb.toString();
    }

    static String parts(PumpingLemma l) {
        if (l instanceof RegularPumpingLemma) {
            RegularPumpingLemma r = (RegularPumpingLemma) l;
            return r.getX() + "|" + r.getY() + "|" + r.getZ();
        }
        ContextFreePumpingLemma c = (ContextFreePumpingLemma) l;
        return c.getU() + "|" + c.getV() + "|" + c.getX() + "|" + c.getY() + "|" + c.getZ();
    }

    static void strings(List<String> acc, String prefix, int left, String alphabet) {
        acc.add(prefix);
        if (left == 0) return;
        for (char c : alphabet.toCharArray()) strings(acc, prefix + c, left - 1, alphabet);
    }

    static void lemma(String kind, int index, PumpingLemma l) throws Exception {
        out.println("LEMMA " + kind + " " + index);
        out.println("title " + l.getTitle());
        out.println("html " + l.getHTMLTitle());
        int[] range = l.getRange();
        out.println("range " + range[0] + " " + range[1]);
        out.println("valid " + l.getPartitionValidity());
        out.println("explanation " + l.getExplanation());
        out.println("ncases " + l.numCasesTotal());
        l.doAll();
        ArrayList<String> descs = l.getDoneDescriptions();
        for (int i = 0; i < descs.size(); i++) out.println("casedesc " + i + " " + descs.get(i));
        l.clearDoneCases();

        // membership of every short string
        List<String> all = new ArrayList<>();
        strings(all, "", 6, "abcd");
        StringBuilder bits = new StringBuilder();
        int nib = 0, cnt = 0;
        for (String s : all) {
            int bit;
            try {
                bit = l.isInLang(s) ? 1 : 0;
            } catch (Throwable t) {
                bit = 0;
                out.println("EXC isInLang [" + s + "] " + t);
            }
            nib = nib * 2 + bit;
            if (++cnt % 4 == 0) {
                bits.append(Integer.toHexString(nib));
                nib = 0;
            }
        }
        while (cnt % 4 != 0) {
            nib *= 2;
            if (++cnt % 4 == 0) bits.append(Integer.toHexString(nib));
        }
        out.println("lang " + bits);

        for (long seed = 1; seed <= 3; seed++) {
            setRandom(seed);
            out.println("SEED " + seed);
            l.reset();
            l.chooseM();
            out.println("chooseM " + l.getM());
            for (int m = range[0]; m <= range[1]; m++) {
                l.setM(m);
                String w = l.getW();
                out.println("m " + m + " w " + w);
                StringBuilder presets = new StringBuilder();
                l.doAll();
                for (int c = 0; c < l.getDoneCases().size(); c++)
                    presets.append(show(((Case) l.getDoneCases().get(c)).getPreset())).append(';');
                l.clearDoneCases();
                if (presets.length() > 0) out.println("presets " + presets);
                try {
                    l.chooseDecomposition();
                    out.println("decomp " + show(l.getDecomposition()) + " parts " + parts(l));
                    l.chooseI();
                    out.println("i " + l.getI());
                    String pumped = l.createPumpedString();
                    out.println("pumped " + pumped + " " + l.isInLang(pumped));
                    out.println("decompstr " + l.getDecompositionAsString());
                } catch (Throwable t) {
                    out.println("EXC choose " + t);
                    l.reset();
                    l.setM(m);
                }
                // lemma-level membership of the generated w
                out.println("winlang " + l.isInLang(l.getW()));
            }
        }

        if (l instanceof ContextFreePumpingLemma && l.numCasesTotal() > 0) {
            // add random cases the way the case panel does
            Random rnd = new Random(99);
            l.clearDoneCases();
            l.reset();
            int m = range[0];
            setRandom(5);
            l.setM(m);
            int len = l.getW().length();
            out.println("CASES m " + m + " w " + l.getW());
            for (int k = 0; k < 60; k++) {
                int[] d = new int[] {rnd.nextInt(len + 1), rnd.nextInt(Math.min(len, m) + 1),
                    rnd.nextInt(Math.min(len, m) + 1), rnd.nextInt(Math.min(len, m) + 1)};
                int i = rnd.nextInt(3);
                try {
                    int ret = l.addCase(d, i);
                    out.println("addcase " + show(d) + " i " + i + " -> " + ret + " done " + l.getDoneCases().size());
                } catch (Throwable t) {
                    out.println("addcase " + show(d) + " i " + i + " -> EXC");
                }
            }
            ArrayList<String> done = l.getDoneDescriptions();
            for (int i = 0; i < done.size(); i++)
                out.println("done " + i + " " + done.get(i) + " input " + show(l.getCase(i).getInput()) + " i " + l.getCase(i).getI());
            l.clearDoneCases();
        }
    }

    // JFLAP saves the chooser whose current lemma is the game being played
    static void save(PumpingLemmaChooser chooser, int index, File file) throws Exception {
        java.lang.reflect.Method m = PumpingLemmaChooser.class.getDeclaredMethod("setCurrent", int.class);
        m.setAccessible(true);
        m.invoke(chooser, index);
        new XMLCodec().encode(chooser, file, null);
    }

    static void gen(File dir) throws Exception {
        RegPumpingLemmaChooser reg = new RegPumpingLemmaChooser();
        CFPumpingLemmaChooser cf = new CFPumpingLemmaChooser();

        // regular, computer first, a^n b^n, a played game
        PumpingLemma a = reg.get(0);
        a.setFirstPlayer("Computer");
        a.setM(5);
        a.setW("aaaaabbbbb");
        a.setDecomposition(new int[] {1, 2});
        a.setI(0);
        a.addAttempt("X = a;   Y = aa;   Z = aaabbbbb; I = 0; <i>Won</i>");
        a.addAttempt("X = λ;   Y = a;   Z = aaaabbbbb; I = 2; <i>Won</i>");
        save(reg, 0, new File(dir, "pumping_reg_anbn.jff"));

        // regular, human first, nothing played yet
        PumpingLemma b = reg.get(6);
        b.setFirstPlayer("Human");
        save(reg, 6, new File(dir, "pumping_reg_fresh.jff"));

        // regular, human first, a valid decomposition of a regular language
        PumpingLemma c = reg.get(6);
        c.setFirstPlayer("Human");
        c.setM(3);
        c.setDecomposition(new int[] {0, 2}, 3);
        save(reg, 6, new File(dir, "pumping_reg_aneven.jff"));

        // context-free, human first, a^n b^n c^n with two cases and an attempt
        PumpingLemma d = cf.get(0);
        d.setFirstPlayer("Human");
        d.setM(4);
        d.setDecomposition(new int[] {0, 1, 3, 1, 7}, 2);
        d.addAttempt("U = λ;   V = a;   X = aaa;   Y = b;   Z = bbbcccc; I = 2; <i>Won</i>");
        d.addCase(new int[] {0, 1, 0, 1}, 2);
        d.addCase(new int[] {7, 1, 0, 1}, 0);
        save(cf, 0, new File(dir, "pumping_cf_anbncn.jff"));

        // context-free, computer first, no cases (languages without a case table)
        PumpingLemma e = cf.get(10);
        e.setFirstPlayer("Computer");
        e.setM(3);
        save(cf, 10, new File(dir, "pumping_cf_fresh.jff"));

        // context-free, human first, fresh
        PumpingLemma f = cf.get(1);
        save(cf, 1, new File(dir, "pumping_cf_ww.jff"));
    }

    static void load(File f) throws Exception {
        PumpingLemma l = (PumpingLemma) new XMLCodec().decode(f, null);
        System.out.println("title " + l.getTitle());
        System.out.println("first " + l.getFirstPlayer());
        System.out.println("m " + l.getM() + " w " + l.getW() + " i " + l.getI());
        System.out.println("decomp " + show(l.getDecomposition()) + " parts " + parts(l));
        for (Object a : l.getAttempts()) System.out.println("attempt " + a);
        for (Object c : l.getDoneCases()) {
            Case k = (Case) c;
            System.out.println("case " + k + " input " + show(k.getInput()) + " i " + k.getI());
        }
    }

    public static void main(String[] args) throws Exception {
        out = new PrintStream(System.out, true, "UTF-8");
        if (args[0].equals("load")) {
            load(new File(args[1]));
            return;
        }
        File dir = new File(args[0]);
        dir.mkdirs();
        PumpingLemmaChooser reg = new RegPumpingLemmaChooser();
        for (int i = 0; i < reg.size(); i++) lemma("reg", i, reg.get(i));
        PumpingLemmaChooser cf = new CFPumpingLemmaChooser();
        for (int i = 0; i < cf.size(); i++) lemma("cf", i, cf.get(i));
        gen(dir);
    }
}
