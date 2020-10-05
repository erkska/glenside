(function() {var implementors = {};
implementors["ndarray"] = [{"text":"impl&lt;A, S, D&gt; Neg for ArrayBase&lt;S, D&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;A: Clone + Neg&lt;Output = A&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;S: DataOwned&lt;Elem = A&gt; + DataMut,<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dimension,&nbsp;</span>","synthetic":false,"types":[]},{"text":"impl&lt;'a, A, S, D&gt; Neg for &amp;'a ArrayBase&lt;S, D&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;&amp;'a A: 'a + Neg&lt;Output = A&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;S: Data&lt;Elem = A&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;D: Dimension,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["num_bigint"] = [{"text":"impl Neg for Sign","synthetic":false,"types":[]},{"text":"impl Neg for BigInt","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Neg for &amp;'a BigInt","synthetic":false,"types":[]},{"text":"impl Neg for BigUint","synthetic":false,"types":[]},{"text":"impl&lt;'a&gt; Neg for &amp;'a BigUint","synthetic":false,"types":[]}];
implementors["num_complex"] = [{"text":"impl&lt;T:&nbsp;Clone + Num + Neg&lt;Output = T&gt;&gt; Neg for Complex&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;'a, T:&nbsp;Clone + Num + Neg&lt;Output = T&gt;&gt; Neg for &amp;'a Complex&lt;T&gt;","synthetic":false,"types":[]}];
implementors["ordered_float"] = [{"text":"impl&lt;T:&nbsp;Float&gt; Neg for OrderedFloat&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl&lt;T:&nbsp;Float&gt; Neg for NotNan&lt;T&gt;","synthetic":false,"types":[]}];
implementors["pyo3"] = [{"text":"impl&lt;'py&gt; Neg for &amp;'py PyComplex","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()