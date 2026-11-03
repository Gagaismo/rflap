import file.XMLCodec;
import grammar.Production;
import grammar.UnboundGrammar;
import grammar.lsystem.Expander;
import grammar.lsystem.LSystem;
import gui.lsystem.Renderer;
import gui.transform.Matrix;
import java.awt.Point;
import java.awt.geom.Rectangle2D;
import java.awt.image.BufferedImage;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.*;

/**
 * L-system harness around JFLAP 7.1's own classes. Run:
 *   java -Dline.separator=$'\n' -Djava.awt.headless=true -cp JFLAP7_1.JAR LHarness.java <out-dir> > lsystem_expected.txt
 * (bash syntax: the files must have LF line ends, JFLAP on Windows writes CR LF otherwise)
 * or, to see how JFLAP reads a file:  ... LHarness.java load <file.jff>
 * It prints expansions, image sizes, final bounds and non-transparent pixels for a few
 * L-systems and saves each of them with JFLAP's own XMLCodec as <out-dir>/lsystem_<name>.jff.
 */
public class LHarness {
    static LSystem make(String axiom, String[][] rules, String[][] params) {
        UnboundGrammar g = new UnboundGrammar();
        for (String[] r : rules) g.addProduction(new Production(r[0], r[1]));
        // the same kind of map the transducer builds
        Map<String, String> map = new HashMap<>();
        for (String[] p : params) map.put(p[0], p[1]);
        return new LSystem(axiom, g, map);
    }

    static String join(List<?> l) {
        StringBuilder sb = new StringBuilder();
        for (Object o : l) {
            if (sb.length() > 0) sb.append(' ');
            sb.append(o);
        }
        return sb.toString();
    }

    static void report(String name, LSystem ls, int levels, long seed, double pitch, double roll, double yaw,
                       boolean pixels, File out) throws Exception {
        System.out.println("CASE " + name);
        Expander ex = new Expander(ls, seed);
        for (int i = 0; i <= levels; i++) {
            System.out.println("expand" + i + " " + join(ex.expansionForLevel(i)));
        }
        List<?> list = ex.expansionForLevel(levels);
        Matrix m = new Matrix();
        m.pitch(pitch);
        m.roll(roll);
        m.yaw(yaw);
        Renderer r = new Renderer();
        Point point = new Point();
        BufferedImage img = (BufferedImage) r.render(list, ls.getValues(), m, null, point);
        Field f = Renderer.class.getDeclaredField("currentTurtle");
        f.setAccessible(true);
        Object turtle = f.get(r);
        Method gb = turtle.getClass().getDeclaredMethod("getBounds");
        gb.setAccessible(true);
        Rectangle2D b = (Rectangle2D) gb.invoke(turtle);
        System.out.println("image " + img.getWidth() + " " + img.getHeight());
        System.out.println("offset " + point.x + " " + point.y);
        System.out.println("bounds " + b.getX() + " " + b.getY() + " " + b.getWidth() + " " + b.getHeight());
        if (pixels) {
            List<String> px = new ArrayList<>();
            for (int y = 0; y < img.getHeight(); y++)
                for (int x = 0; x < img.getWidth(); x++) {
                    int argb = img.getRGB(x, y);
                    if ((argb >>> 24) != 0) px.add(x + " " + y + " " + Integer.toHexString(argb));
                }
            System.out.println("pixels " + px.size());
            for (String s : px) System.out.println("px " + s);
        }
        if (out != null) new XMLCodec().encode(ls, new File(out, "lsystem_" + name + ".jff"), null);
    }

    static void load(File f) {
        LSystem ls = (LSystem) new XMLCodec().decode(f, null);
        System.out.println("axiom " + join(ls.getAxiom()));
        for (Object s : ls.getSymbolsWithReplacements()) {
            for (List r : ls.getReplacements((String) s)) System.out.println("rule [" + s + "] -> " + join(r));
        }
        System.out.println("values " + new TreeMap<>(ls.getValues()));
        System.out.println("nondeterministic " + ls.nondeterministic());
    }

    public static void main(String[] args) throws Exception {
        if (args[0].equals("load")) {
            load(new File(args[1]));
            return;
        }
        File out = new File(args[0]);
        out.mkdirs();
        report("koch",
            make("g - - g - - g", new String[][] {{"g", "g + g - - g + g"}},
                new String[][] {{"angle", "60"}, {"distance", "5"}}),
            2, 0, 0, 0, 0, false, out);
        report("plant",
            make("g [ + g ] g", new String[][] {},
                new String[][] {{"angle", "90"}, {"distance", "10"}}),
            0, 0, 0, 0, 0, true, out);
        report("bush",
            make("x", new String[][] {{"x", "g [ + x ] [ - x ] g x"}, {"g", "g g"}},
                new String[][] {{"angle", "25"}, {"distance", "4"}, {"lineWidth", "2"}, {"lineIncrement", "0.5"}}),
            3, 0, 0, 0, 0, false, out);
        report("colors",
            make("g color=red lineWidth=3 g [ color(0,0,255) ! g ] g # g ## g polygonColor=forestGreen { g + g + g } g",
                new String[][] {},
                new String[][] {{"angle", "90"}, {"distance", "10"}, {"hueChange", "60"}}),
            0, 0, 0, 0, 0, true, out);
        report("solid3d",
            make("/ & g g(2*3) distance=4 & g ^ ^ g * g % g",
                new String[][] {},
                new String[][] {{"angle", "90"}, {"distance", "10"}}),
            0, 0, 0, 0, 0, false, out);
        report("viewed3d",
            make("g + g & g", new String[][] {},
                new String[][] {{"angle", "30"}, {"distance", "10"}}),
            0, 0, 20, 10, 40, false, out);
        report("stochastic",
            make("a", new String[][] {{"a", "a b a"}, {"a", "b a"}, {"b", "b b"}, {"b", "a"}},
                new String[][] {}),
            6, 7, 0, 0, 0, false, out);
        report("context",
            make("a b a b b a", new String[][] {{"1 a b", "c"}, {"0 b", "b"}, {"2 a b a", "d"}},
                new String[][] {}),
            3, 3, 0, 0, 0, false, out);
        report("empties",
            make("", new String[][] {{"a", ""}, {"b", "b  b"}},
                new String[][] {{"angle", ""}, {"zz", "1"}}),
            1, 0, 0, 0, 0, false, out);
        report("params",
            make("g(a*2) +(a) g(a) g", new String[][] {},
                new String[][] {{"a", "7"}, {"b", "a+1"}, {"angle", "b*10"}, {"distance", "b"}, {"c", "angle/2"}}),
            0, 0, 0, 0, 0, false, out);
        // expression evaluator: each expression is the length of a forward move
        String[] exprs = {"2*3", "10-4", "2^3", "(1+2)*4", "-5", "7/2", "zzz", "a*2", "a-1", "2^3^2", "8/2/2",
            "1+2*3", "2*3+1", "(2+3)(4)", "3-2-1", "2*-3", "1.5+.5", "((2))", "2^-1", "1e2", "-a"};
        for (int i = 0; i < exprs.length; i++) {
            report("expr" + i,
                make("g(" + exprs[i] + ")", new String[][] {}, new String[][] {{"a", "3"}, {"distance", "1"}}),
                0, 0, 0, 0, 0, false, null);
            System.out.println("expr " + exprs[i]);
        }
    }
}
