// Ввод игрока: клавиатура, мышь и геймпад. Работает и с новым Input System, и со старым Input Manager.
using UnityEngine;
#if ENABLE_INPUT_SYSTEM
using UnityEngine.InputSystem;
#endif

namespace CarBang
{
    public enum Btn { Fire, Oil, Nitro, Handbrake, Reset, Restart, Pause, LookBack, Camera, Help, Horn }

    public static class GameInput
    {
        public static float Throttle()
        {
            float v = 0f;
#if ENABLE_INPUT_SYSTEM
            var k = Keyboard.current;
            if (k != null && (k.wKey.isPressed || k.upArrowKey.isPressed)) v = 1f;
            var g = Gamepad.current;
            if (g != null) v = Mathf.Max(v, g.rightTrigger.ReadValue());
#elif ENABLE_LEGACY_INPUT_MANAGER
            if (Input.GetKey(KeyCode.W) || Input.GetKey(KeyCode.UpArrow)) v = 1f;
#endif
            return v;
        }

        public static float Brake()
        {
            float v = 0f;
#if ENABLE_INPUT_SYSTEM
            var k = Keyboard.current;
            if (k != null && (k.sKey.isPressed || k.downArrowKey.isPressed)) v = 1f;
            var g = Gamepad.current;
            if (g != null) v = Mathf.Max(v, g.leftTrigger.ReadValue());
#elif ENABLE_LEGACY_INPUT_MANAGER
            if (Input.GetKey(KeyCode.S) || Input.GetKey(KeyCode.DownArrow)) v = 1f;
#endif
            return v;
        }

        public static float Steer()
        {
            float v = 0f;
#if ENABLE_INPUT_SYSTEM
            var k = Keyboard.current;
            if (k != null)
            {
                if (k.aKey.isPressed || k.leftArrowKey.isPressed) v -= 1f;
                if (k.dKey.isPressed || k.rightArrowKey.isPressed) v += 1f;
            }
            var g = Gamepad.current;
            if (g != null)
            {
                float x = g.leftStick.ReadValue().x;
                if (Mathf.Abs(x) > 0.08f) v = Mathf.Clamp(v + (Mathf.Abs(x) - 0.08f) / 0.92f * Mathf.Sign(x), -1f, 1f);
            }
#elif ENABLE_LEGACY_INPUT_MANAGER
            if (Input.GetKey(KeyCode.A) || Input.GetKey(KeyCode.LeftArrow)) v -= 1f;
            if (Input.GetKey(KeyCode.D) || Input.GetKey(KeyCode.RightArrow)) v += 1f;
#endif
            return v;
        }

        public static bool Held(Btn b)
        {
#if ENABLE_INPUT_SYSTEM
            var k = Keyboard.current;
            var m = Mouse.current;
            var g = Gamepad.current;
            switch (b)
            {
                case Btn.Fire: return (m != null && m.leftButton.isPressed) || (k != null && (k.leftCtrlKey.isPressed || k.jKey.isPressed)) || (g != null && g.buttonWest.isPressed);
                case Btn.Nitro: return (k != null && k.leftShiftKey.isPressed) || (g != null && (g.buttonEast.isPressed || g.leftShoulder.isPressed));
                case Btn.Handbrake: return (k != null && k.spaceKey.isPressed) || (g != null && g.buttonSouth.isPressed);
                case Btn.LookBack: return (k != null && k.vKey.isPressed) || (g != null && g.dpad.left.isPressed);
                default: return false;
            }
#elif ENABLE_LEGACY_INPUT_MANAGER
            switch (b)
            {
                case Btn.Fire: return Input.GetMouseButton(0) || Input.GetKey(KeyCode.LeftControl) || Input.GetKey(KeyCode.J);
                case Btn.Nitro: return Input.GetKey(KeyCode.LeftShift);
                case Btn.Handbrake: return Input.GetKey(KeyCode.Space);
                case Btn.LookBack: return Input.GetKey(KeyCode.V);
                default: return false;
            }
#else
            return false;
#endif
        }

        public static bool Pressed(Btn b)
        {
#if ENABLE_INPUT_SYSTEM
            var k = Keyboard.current;
            var g = Gamepad.current;
            switch (b)
            {
                case Btn.Oil: return (k != null && (k.qKey.wasPressedThisFrame || k.lKey.wasPressedThisFrame)) || (g != null && (g.dpad.down.wasPressedThisFrame || g.rightShoulder.wasPressedThisFrame));
                case Btn.Reset: return (k != null && k.rKey.wasPressedThisFrame) || (g != null && g.selectButton.wasPressedThisFrame);
                case Btn.Restart: return (k != null && k.enterKey.wasPressedThisFrame) || (g != null && g.buttonNorth.wasPressedThisFrame);
                case Btn.Pause: return (k != null && (k.escapeKey.wasPressedThisFrame || k.pKey.wasPressedThisFrame)) || (g != null && g.startButton.wasPressedThisFrame);
                case Btn.Camera: return (k != null && k.cKey.wasPressedThisFrame) || (g != null && g.rightStickButton.wasPressedThisFrame);
                case Btn.Help: return k != null && k.f1Key.wasPressedThisFrame;
                case Btn.Horn: return (k != null && k.hKey.wasPressedThisFrame) || (g != null && g.leftStickButton.wasPressedThisFrame);
                default: return false;
            }
#elif ENABLE_LEGACY_INPUT_MANAGER
            switch (b)
            {
                case Btn.Oil: return Input.GetKeyDown(KeyCode.Q) || Input.GetKeyDown(KeyCode.L);
                case Btn.Reset: return Input.GetKeyDown(KeyCode.R);
                case Btn.Restart: return Input.GetKeyDown(KeyCode.Return);
                case Btn.Pause: return Input.GetKeyDown(KeyCode.Escape) || Input.GetKeyDown(KeyCode.P);
                case Btn.Camera: return Input.GetKeyDown(KeyCode.C);
                case Btn.Help: return Input.GetKeyDown(KeyCode.F1);
                case Btn.Horn: return Input.GetKeyDown(KeyCode.H);
                default: return false;
            }
#else
            return false;
#endif
        }
    }
}
