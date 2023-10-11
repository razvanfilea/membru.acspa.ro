import React, {ReactElement, useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {Button, Grid, Group, Overlay, Space, Stack, Text, Title} from "@mantine/core";
import 'dayjs/locale/ro'
import {Location, LocationName, MemberTypes} from "../types/wrapper";
import {useRouter} from "next/router";
import {addDaysToDate, dateToISOString} from "../utils/date";
import {ConfirmSelection, GeneralInfoPopup} from "../components/MainPageComponents";
import {Database} from "../types/database.types";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import {DatePicker} from "@mantine/dates";
import useProfileData from "../hooks/useProfileData";
import SelectGameTable from "../components/MainPageComponents/SelectGameTable";
import useGlobalVars from "../hooks/useGlobalVars";
import Link from "next/link";

interface IParams {
    gara: Location
    boromir: Location
    daysAhead: number
}

export default function MakeReservationPage(params: IParams): ReactElement {
    const router = useRouter()
    const profileData = useProfileData()
    const {data: globalVars} = useGlobalVars()

    const [locationName, /*setLocationName*/] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date>(new Date)
    const [selectedStartHour, setSelectedStartHour] = useState<number | null>(null)

    const selectedDateISO = useMemo(() => dateToISOString(selectedDate), [selectedDate])

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedStartHour(null)
    }

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile == null) {
            const timer = setTimeout(() => {
                router.push('/login').then(null)
            }, 400)

            return () => clearTimeout(timer)
        }
    }, [profileData, router])

    const location = locationName == LocationName.Gara ? params.gara : params.boromir;

    return <>
        <Head>
            <title>Rezervări - ACSPA</title>
        </Head>

        <Group position={'apart'} align={'center'}>
            <Title>Rezervări</Title>
            {globalVars?.entrance_code &&
                <Text size={'lg'}>Cod intrare: <b>{globalVars?.entrance_code}</b></Text>
            }
        </Group>

        <GeneralInfoPopup/>

        <Grid
            grow={true}
            columns={4}>

            <Grid.Col span={'auto'}>
                <Text>Alege ziua rezervării:</Text>

                {!profileData.isLoading && profileData.profile != null &&
                    <DatePicker
                        minDate={new Date}
                        maxDate={addDaysToDate(new Date, params.daysAhead)}
                        hideOutsideDates={true}
                        maxLevel={'month'}
                        size={"lg"}
                        locale={"ro"}
                        value={selectedDate}
                        onChange={(date) => {
                            if (profileData.profile != null && date != null)
                                onSelectedDateChange(date)
                        }}
                        getDayProps={(date) => {
                            if (date.getDate() === (new Date).getDate()
                                && date.getMonth() === (new Date).getMonth()
                                && date.getDate() !== selectedDate?.getDate()) {
                                return {
                                    sx: (theme) => ({
                                        backgroundColor: theme.colors.blue[7],
                                        color: theme.white
                                    })
                                };
                            }
                            return {};
                        }}
                        withCellSpacing={true}
                    />
                }
            </Grid.Col>

            <Grid.Col span={2}>
                <Stack>
                    <SelectGameTable location={location} selectedDateISO={selectedDateISO}
                                     selectedStartHour={selectedStartHour} onSetStartHour={setSelectedStartHour}/>

                    {ConfirmSelection(location, selectedDateISO, selectedStartHour)}
                </Stack>
            </Grid.Col>
        </Grid>

        <Space h="xl"/>

        {globalVars?.maintenance == true &&
            <Overlay center={true} opacity={0.85}>
                <Stack p={'md'}>
                    <Title>Site-ul este în mentenanță.</Title>
                    <Space h={'lg'}/>
                    <Text>Vă rugăm reveniți mai târziu</Text>

                    {profileData.profile?.role === MemberTypes.Fondator &&
                        <Link href={"/admin"} passHref={true}>
                            <Button color={'red'}>Panou Fondator</Button>
                        </Link>
                    }

                </Stack>
            </Overlay>
        }
    </>;
}

export async function getStaticProps({}) {
    const supabase = createPagesBrowserClient<Database>()

    const {data: locations} = await supabase.from('locations').select()
    const garaLocation = locations!.find(value => value.name == LocationName.Gara)
    const boromirLocation = locations!.find(value => value.name == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: garaLocation!,
        boromir: boromirLocation!
    }

    return {props}
}
