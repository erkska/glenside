(function() {var implementors = {};
implementors["env_logger"] = [{"text":"impl&lt;'a, T:&nbsp;UpperHex&gt; UpperHex for StyledValue&lt;'a, T&gt;","synthetic":false,"types":[]}];
implementors["itertools"] = [{"text":"impl&lt;'a, I&gt; UpperHex for Format&lt;'a, I&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;I: Iterator,<br>&nbsp;&nbsp;&nbsp;&nbsp;I::Item: UpperHex,&nbsp;</span>","synthetic":false,"types":[]}];
implementors["num_bigint"] = [{"text":"impl UpperHex for BigInt","synthetic":false,"types":[]},{"text":"impl UpperHex for BigUint","synthetic":false,"types":[]}];
implementors["num_complex"] = [{"text":"impl&lt;T&gt; UpperHex for Complex&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: UpperHex + Num + PartialOrd + Clone,&nbsp;</span>","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()