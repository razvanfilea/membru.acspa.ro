import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import {ReactElement, useState} from "react";
import {Button, Card, NumberInput, Space, Stack, Switch, Title} from "@mantine/core";
import useGlobalVars from "../../hooks/useGlobalVars";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import {useRouter} from "next/router";

export default function WebsiteSettingsPage(): ReactElement {
    useExitIfNotFounder();

    const router = useRouter()
    const supabase = useSupabaseClient<Database>()

    const [maintenanceMode, setMaintenanceMode] = useState(false);
    const [entranceCode, setEntranceCode] = useState<number | string>("");
    useGlobalVars((vars) => {
        setMaintenanceMode(vars.maintenance)
        setEntranceCode(vars.entrance_code)
    });

    return <Card style={{margin: `var(--mantine-spacing-md)`}}>
        <Stack gap={'xl'}>
            <Title>Setări website</Title>

            <Switch
                size={'lg'} label={"Mod mentenanță"}
                checked={maintenanceMode}
                onChange={(event) => setMaintenanceMode(event.currentTarget.checked)}/>

            <NumberInput size={'lg'} label={"Cod intrare:"} hideControls={true} value={entranceCode} onChange={setEntranceCode}/>

            <Space h={'lg'} />

            <Button size={'lg'} onClick={async () => {
                await supabase.from('global_vars')
                    .update({
                        maintenance: maintenanceMode,
                        entrance_code: typeof entranceCode === 'string' ? undefined : entranceCode
                    })
                    .gte('maintenance', 0) // Workaround for supabase not allowing update without a where clause
                router.back()
            }}>Aplică setările</Button>

        </Stack>

    </Card>
}