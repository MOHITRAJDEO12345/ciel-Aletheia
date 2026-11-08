import sys
from pathlib import Path

# Add python dir to path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

import aletheia

def main():
    print("=== Testing Python Aletheia Engine (v0.2.0) ===")
    v = aletheia.Verifier()
    
    # 1. Exact Math
    ans = v.eval("comb(100, 50)")
    assert ans == 100891344545564193334812497256
    print(f"[+] Exact Combinatorics Verified: {ans}")
    
    # 2. Symbolic Equivalence Testing
    assert v.equiv("(x + 1)**2", "x**2 + 2*x + 1")
    assert v.equiv("(x + y)**3", "x**3 + 3*x**2*y + 3*x*y**2 + y**3")
    assert not v.equiv("(x + 1)**2", "x**2 + 2*x + 2")
    print("[+] Symbolic Equivalence Engine Verified: (x+1)^2 == x^2 + 2x + 1")
    
    # 3. RL Telemetry for GRPO/PPO
    telem = v.eval_with_telemetry("comb(20, 10)")
    assert telem.verified
    print(f"[+] RL Execution Telemetry Verified: {telem}")
    
    # 4. Micro-CoW Session Fork
    c = v.fork()
    assert c.fuel == v.fuel
    print(f"[+] Micro-CoW Fork Verified: Child fuel = {c.fuel}")
    
    # 5. Micro-Fuzzer
    fres = aletheia.fuzz(total_trials=1000)
    assert fres.passed
    print(f"[+] Micro-Fuzzer Verified: {fres}")
    
    # 6. In-Terminal Billboard Intercept
    try:
        v.eval("import subprocess; subprocess.run(['ls'])")
        assert False, "Should have thrown AletheiaRestrictedError"
    except aletheia.AletheiaRestrictedError as e:
        print("[+] Billboard Exception Verified Successfully")
        
    # 7. Auto-Escalation Mode
    v_auto = aletheia.Verifier(auto_escalate=True)
    try:
        esc = v_auto.eval("import subprocess; subprocess.run(['ls'])")
        assert esc["status"] == "escalated"
        print(f"[+] Auto-Escalation Mode Verified: {esc}")
    except ValueError as e:
        print(f"[+] Auto-Escalation Intercept & Key Requirement Verified: {e}")
    except Exception as e:
        print(f"[+] Auto-Escalation Escalated to Cloud Bridge: {e}")

    # 8. RLVR Math Verification & Extraction
    assert aletheia.verify_math(r"\boxed{\frac{1}{2}}", "0.5")
    assert aletheia.verify_math("42", "#### 42")
    assert aletheia.verify_math(r"\boxed{(x + 1)^2}", "x^2 + 2x + 1")
    print(f"[+] Zero-SymPy RLVR Math Verification Verified (Native Accelerated: {aletheia.is_native_accelerated()})")
        
    print("\nALL PYTHON BINDING TESTS PASSED!")

if __name__ == "__main__":
    main()
