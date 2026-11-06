import sys
from pathlib import Path

# Add python dir to path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import aletheia

def main():
    print("=== Testing Python Aletheia Engine ===")
    v = aletheia.Verifier()
    
    # 1. Exact Math
    ans = v.eval("comb(100, 50)")
    assert ans == 100891344545564193334812497256
    print(f"[+] Exact Combinatorics Verified: {ans}")
    
    # 2. Micro-CoW Session Fork
    c = v.fork()
    assert c.fuel == v.fuel
    print(f"[+] Micro-CoW Fork Verified: Child fuel = {c.fuel}")
    
    # 3. Micro-Fuzzer
    fres = aletheia.fuzz(total_trials=1000)
    assert fres.passed
    print(f"[+] Micro-Fuzzer Verified: {fres}")
    
    # 4. In-Terminal Billboard Intercept
    try:
        v.eval("import subprocess; subprocess.run(['ls'])")
        assert False, "Should have thrown AletheiaRestrictedError"
    except aletheia.AletheiaRestrictedError as e:
        print("[+] Billboard Exception Verified Successfully:")
        print(str(e))
        
    print("\nALL PYTHON BINDING TESTS PASSED!")

if __name__ == "__main__":
    main()

